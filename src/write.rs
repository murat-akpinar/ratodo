//! `Doc` -> `todo.md`, and the file IO around it.
//! See docs/architecture.md#concurrent-editing.

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

use crate::model::Doc;
use crate::parse;

pub struct Loaded {
    pub doc: Doc,
    /// `None` when the file did not exist yet.
    pub mtime: Option<SystemTime>,
}

/// For an untouched document this returns the input byte for byte.
pub fn render(doc: &Doc) -> String {
    let mut out = String::new();
    for line in &doc.lines {
        out.push_str(&line.text());
        out.push_str(line.ending.as_str());
    }
    out
}

/// A missing file is an empty document, not an error.
pub fn load(path: &Path) -> Result<Loaded> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let mtime = fs::metadata(path).and_then(|m| m.modified()).ok();
            Ok(Loaded {
                doc: parse::parse(&text),
                mtime,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Loaded {
            doc: Doc::default(),
            mtime: None,
        }),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// Refuses if the file changed since `load` reported `read_mtime`: someone else
/// has written to it and overwriting would throw that away.
///
/// `backup_dir` is where the `.bak` goes — never beside the list. See
/// docs/format.md.
pub fn save(
    path: &Path,
    doc: &Doc,
    read_mtime: Option<SystemTime>,
    backup_dir: &Path,
) -> Result<()> {
    check_unchanged(path, read_mtime)?;

    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        fs::create_dir_all(dir)
            .with_context(|| format!("creating the directory {}", dir.display()))?;
    }

    // A dotfiles user's todo.md is usually a symlink into their repo. Renaming
    // over the link would replace it with a plain file and quietly detach it.
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let existing = fs::metadata(&target).ok();

    if existing.is_some() {
        let backup = backup_path(backup_dir, &target);
        fs::create_dir_all(backup_dir)
            .with_context(|| format!("creating the directory {}", backup_dir.display()))?;
        fs::copy(&target, &backup)
            .with_context(|| format!("writing the backup {}", backup.display()))?;
    }

    let tmp = temp_path(&target);
    if let Err(e) = write_temp(&tmp, &render(doc), existing.as_ref()) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    fs::rename(&tmp, &target).with_context(|| format!("replacing {}", target.display()))?;

    // Without syncing the directory the rename itself can be lost on a crash.
    // Best effort: some filesystems refuse to open a directory.
    if let Some(dir) = target.parent()
        && let Ok(handle) = File::open(dir)
    {
        let _ = handle.sync_all();
    }

    Ok(())
}

fn write_temp(tmp: &Path, text: &str, existing: Option<&fs::Metadata>) -> Result<()> {
    let mut file = File::create(tmp).with_context(|| format!("creating {}", tmp.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("writing {}", tmp.display()))?;
    file.sync_all()
        .with_context(|| format!("flushing {}", tmp.display()))?;
    drop(file);

    // A list the user chmodded to 600 must not come back as 644.
    if let Some(meta) = existing {
        fs::set_permissions(tmp, meta.permissions())
            .with_context(|| format!("restoring the mode of {}", tmp.display()))?;
    }
    Ok(())
}

/// Somebody else wrote the file first. Its own type because it is the one
/// failure that is not a fault: the TUI answers it with a line offering a
/// reload, where any other error is a reason to stop.
#[derive(Debug)]
pub struct Conflict(pub PathBuf);

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} changed on disk since it was read — nothing was written.\n\
             Re-run to pick up the new version.",
            self.0.display()
        )
    }
}

impl std::error::Error for Conflict {}

fn check_unchanged(path: &Path, read_mtime: Option<SystemTime>) -> Result<()> {
    let current = fs::metadata(path).and_then(|m| m.modified()).ok();
    if current != read_mtime {
        return Err(Conflict(path.to_path_buf()).into());
    }
    Ok(())
}

/// The full target path, flattened into one file name. A single `todo.md.bak`
/// slot would mean the backup of one `--file` list quietly replacing another's.
///
/// Everything Windows forbids in a name goes, not only the separators: there
/// `canonicalize` answers with a verbatim `\\?\C:\…` path, and a `?` or a `:`
/// left in the slug is os error 123 on every write past the first.
///
/// Both separators are named rather than asked of `std::path::is_separator`,
/// which answers for the host: a Windows path slugged on Linux would keep its
/// backslashes, and the test below could then only run on one of the two.
fn backup_path(dir: &Path, target: &Path) -> PathBuf {
    let slug = target
        .to_string_lossy()
        .split(|c: char| r#"/\:?*"<>|"#.contains(c))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    dir.join(format!("{slug}.bak"))
}

/// Next to the target so the rename stays on one filesystem; the pid keeps two
/// concurrent `ratodo add` calls apart.
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Due, State, Task};
    use chrono::NaiveDate;

    #[test]
    fn render_is_the_inverse_of_parse() {
        for input in [
            "",
            "\n",
            "- [ ] a\n",
            "- [ ] a",
            "- [ ] a\r\n- [x] b\r\n",
            "## Work\n\n- [ ] a\n\n> quote\n---\n",
            "mixed\r\nendings\nin one file\n",
            "trailing spaces   \n\ttab indented\n",
        ] {
            assert_eq!(render(&parse::parse(input)), input, "input: {input:?}");
        }
    }

    /// Ticking changes the checkbox byte and appends the stamp. Everything
    /// between the two — the user's four-space gaps and the order they put
    /// their own fields in — is not ours to tidy on the way past.
    #[test]
    fn ticking_changes_one_byte_and_appends_the_stamp() {
        let before = "- [ ]    spaced   out   title   @2026-08-10   #tag\n";
        let mut doc = parse::parse(before);
        doc.tasks_mut()
            .next()
            .unwrap()
            .set_state(State::Done, a_day());

        assert_eq!(
            render(&doc),
            "- [x]    spaced   out   title   @2026-08-10   #tag ✓2026-08-10\n"
        );
    }

    /// The stamp is the tick's, so unticking takes it away again — including the
    /// space it was given. A line that goes out and comes back has to be the
    /// line that went out.
    #[test]
    fn toggling_back_and_forth_returns_the_original() {
        for before in [
            "  * [ ] weirdly written but mine\n",
            "- [ ] with   odd   spacing   @2026-08-12 16:00 #a #b !low\n",
            "- [-] and one that was cancelled\n",
        ] {
            let mut doc = parse::parse(before);
            let was = doc.tasks().next().unwrap().state;
            doc.tasks_mut()
                .next()
                .unwrap()
                .set_state(State::Done, a_day());
            doc.tasks_mut().next().unwrap().set_state(was, a_day());
            assert_eq!(render(&doc), before);
        }
    }

    /// The one direction that does not come back unchanged, and on purpose: a
    /// task finished before the stamp existed — or by hand in `vim` — gets one
    /// the next time it is ticked. Pinned so it cannot happen by accident.
    #[test]
    fn re_ticking_an_unstamped_task_stamps_it() {
        let mut doc = parse::parse("  * [X] finished before the stamp existed\n");
        doc.tasks_mut()
            .next()
            .unwrap()
            .set_state(State::Open, a_day());
        doc.tasks_mut()
            .next()
            .unwrap()
            .set_state(State::Done, a_day());
        assert_eq!(
            render(&doc),
            "  * [x] finished before the stamp existed ✓2026-08-10\n"
        );
    }

    /// Putting a date off moves the date and leaves the time, the tags and the
    /// spacing exactly where they were.
    #[test]
    fn postponing_moves_the_date_and_nothing_else() {
        let mut doc = parse::parse("- [ ] a   task @2026-08-10 16:00 #tag !high\n");
        doc.tasks_mut()
            .next()
            .unwrap()
            .postpone(NaiveDate::from_ymd_opt(2026, 8, 17).unwrap());
        assert_eq!(
            render(&doc),
            "- [ ] a   task @2026-08-17 16:00 #tag !high\n"
        );
    }

    /// An undated task is the case with nothing to replace, and `p` on one is a
    /// reasonable thing to do. The date is appended rather than refused.
    #[test]
    fn postponing_an_undated_task_gives_it_a_date() {
        let mut doc = parse::parse("- [ ] no date here #tag\n");
        doc.tasks_mut()
            .next()
            .unwrap()
            .postpone(NaiveDate::from_ymd_opt(2026, 8, 17).unwrap());
        assert_eq!(render(&doc), "- [ ] no date here #tag @2026-08-17\n");
    }

    fn a_day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
    }

    #[test]
    fn appending_to_a_file_without_a_final_newline() {
        let mut doc = parse::parse("- [ ] first");
        doc.push_task(Task::new(
            State::Open,
            "second".into(),
            Some(Due::new(NaiveDate::from_ymd_opt(2026, 8, 11).unwrap())),
            vec!["home".into()],
            None,
        ));
        assert_eq!(
            render(&doc),
            "- [ ] first\n- [ ] second @2026-08-11 #home\n"
        );
    }

    #[test]
    fn appending_to_an_empty_document() {
        let mut doc = Doc::default();
        doc.push_task(Task::new(State::Open, "first".into(), None, vec![], None));
        assert_eq!(render(&doc), "- [ ] first\n");
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("ratodo-test-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_missing_file_loads_as_an_empty_document() {
        let dir = TempDir::new("missing");
        let loaded = load(&dir.file("todo.md")).unwrap();
        assert_eq!(loaded.doc.task_count(), 0);
        assert!(loaded.mtime.is_none());
    }

    #[test]
    fn saving_creates_the_directory_and_the_file() {
        let dir = TempDir::new("create");
        let path = dir.file("nested/todo.md");
        let mut doc = Doc::default();
        doc.push_task(Task::new(State::Open, "first".into(), None, vec![], None));

        save(&path, &doc, None, &dir.file("bak")).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] first\n");
        assert!(!path.with_extension("md.bak").exists());
    }

    #[test]
    fn a_backup_is_taken_before_overwriting() {
        let dir = TempDir::new("backup");
        let path = dir.file("todo.md");
        let bak = dir.file("bak");
        fs::write(&path, "- [ ] original\n").unwrap();

        let loaded = load(&path).unwrap();
        let mut doc = loaded.doc;
        doc.push_task(Task::new(State::Open, "second".into(), None, vec![], None));
        save(&path, &doc, loaded.mtime, &bak).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] original\n- [ ] second\n"
        );
        assert_eq!(
            fs::read_to_string(backup_path(&bak, &fs::canonicalize(&path).unwrap())).unwrap(),
            "- [ ] original\n"
        );
    }

    /// The list is usually symlinked into a dotfiles repo; a `.bak` beside it
    /// leaves `git status` dirty after every single capture.
    #[test]
    fn no_backup_is_ever_written_beside_the_list() {
        let dir = TempDir::new("bak-elsewhere");
        let path = dir.file("todo.md");
        fs::write(&path, "- [ ] original\n").unwrap();

        let loaded = load(&path).unwrap();
        let mut doc = loaded.doc;
        doc.push_task(Task::new(State::Open, "second".into(), None, vec![], None));
        save(&path, &doc, loaded.mtime, &dir.file("bak")).unwrap();

        let beside: Vec<_> = fs::read_dir(&dir.0)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".bak"))
            .collect();
        assert!(beside.is_empty(), "found {beside:?} next to the list");
    }

    /// Two lists reached through `--file` must not share one backup slot, or the
    /// second capture of the day silently destroys the first list's insurance.
    #[test]
    fn two_lists_get_two_backups() {
        let dir = TempDir::new("two-lists");
        let bak = dir.file("bak");
        let work = dir.file("work/todo.md");
        let home = dir.file("home/todo.md");

        for (path, body) in [(&work, "- [ ] at work\n"), (&home, "- [ ] at home\n")] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
            let loaded = load(path).unwrap();
            let mut doc = loaded.doc;
            doc.push_task(Task::new(State::Open, "second".into(), None, vec![], None));
            save(path, &doc, loaded.mtime, &bak).unwrap();
        }

        let mut backups: Vec<String> = fs::read_dir(&bak)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| fs::read_to_string(e.path()).unwrap())
            .collect();
        backups.sort();
        assert_eq!(backups, ["- [ ] at home\n", "- [ ] at work\n"]);
    }

    #[test]
    fn a_changed_file_is_never_overwritten() {
        let dir = TempDir::new("conflict");
        let path = dir.file("todo.md");
        fs::write(&path, "- [ ] mine\n").unwrap();
        let loaded = load(&path).unwrap();

        // Somebody else writes while we were thinking.
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&path, "- [ ] theirs\n").unwrap();

        let err = save(&path, &loaded.doc, loaded.mtime, &dir.file("bak")).unwrap_err();
        assert!(err.to_string().contains("changed on disk"), "{err}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] theirs\n");
    }

    /// Every write after the first takes a backup, so a name the filesystem
    /// refuses is the whole tool broken rather than one lost `.bak`.
    ///
    /// Windows hands `canonicalize` a verbatim path — `\\?\C:\…` — and both `?`
    /// and `:` are illegal in a file name there. Flattening only the separators
    /// left them in, and every capture past the first died on os error 123.
    #[test]
    fn a_backup_name_holds_nothing_a_filesystem_refuses() {
        let windows = backup_path(Path::new("bak"), Path::new(r"\\?\C:\Users\you\todo.md"));
        let name = windows.file_name().unwrap().to_string_lossy();
        assert_eq!(
            name, "C-Users-you-todo.md.bak",
            "still unwritable on Windows"
        );

        let unix = backup_path(Path::new("bak"), Path::new("/home/you/todo.md"));
        assert_eq!(
            unix.file_name().unwrap().to_string_lossy(),
            "home-you-todo.md.bak",
            "the name every existing backup already has"
        );
    }

    #[test]
    fn no_temp_file_is_left_behind() {
        let dir = TempDir::new("tmp");
        let path = dir.file("todo.md");
        save(&path, &Doc::default(), None, &dir.file("bak")).unwrap();
        assert!(!temp_path(&path).exists());
    }

    #[test]
    #[cfg(unix)]
    fn a_symlinked_list_stays_a_symlink() {
        let dir = TempDir::new("symlink");
        let real = dir.file("dotfiles-todo.md");
        let link = dir.file("todo.md");
        fs::write(&real, "- [ ] original\n").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let loaded = load(&link).unwrap();
        let mut doc = loaded.doc;
        doc.push_task(Task::new(State::Open, "second".into(), None, vec![], None));
        save(&link, &doc, loaded.mtime, &dir.file("bak")).unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the symlink was replaced by a regular file"
        );
        assert_eq!(
            fs::read_to_string(&real).unwrap(),
            "- [ ] original\n- [ ] second\n",
            "the write did not reach the file the link points at"
        );
    }

    #[test]
    #[cfg(unix)]
    fn file_permissions_survive_a_write() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("mode");
        let path = dir.file("todo.md");
        fs::write(&path, "- [ ] private\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

        let loaded = load(&path).unwrap();
        let mut doc = loaded.doc;
        doc.push_task(Task::new(State::Open, "second".into(), None, vec![], None));
        save(&path, &doc, loaded.mtime, &dir.file("bak")).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "a 0600 list came back as {mode:o}");
    }
}
