// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;

use crate::common::TestEnvironment;

fn get_log_output(test_env: &TestEnvironment, cwd: &Path) -> String {
    let template = r#"separate(" ", change_id.short(), empty, description, local_branches)"#;
    test_env.jj_cmd_success(cwd, &["log", "-T", template])
}

fn get_recorded_dates(test_env: &TestEnvironment, cwd: &Path, revset: &str) -> String {
    let template = r#"separate("\n", "Author date:  " ++ author.timestamp(), "Committer date: " ++ committer.timestamp())"#;
    test_env.jj_cmd_success(cwd, &["log", "--no-graph", "-T", template, "-r", revset])
}

#[test]
fn test_split_by_paths() {
    let mut test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    std::fs::write(repo_path.join("file1"), "foo").unwrap();
    std::fs::write(repo_path.join("file2"), "foo").unwrap();
    std::fs::write(repo_path.join("file3"), "foo").unwrap();

    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r###"
    @  qpvuntsmwlqt false
    ◉  zzzzzzzzzzzz true
    "###);
    insta::assert_snapshot!(get_recorded_dates(&test_env, &repo_path,"@"), @r###"
    Author date:  2001-02-03 04:05:07.000 +07:00
    Committer date: 2001-02-03 04:05:08.000 +07:00
    "###);

    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(
        edit_script,
        ["dump editor0", "next invocation\n", "dump editor1"].join("\0"),
    )
    .unwrap();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["split", "file2"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    First part: qpvuntsm d62c056f (no description set)
    Second part: zsuskuln 5a32af4a (no description set)
    Working copy now at: zsuskuln 5a32af4a (no description set)
    Parent commit      : qpvuntsm d62c056f (no description set)
    "###);
    insta::assert_snapshot!(
        std::fs::read_to_string(test_env.env_root().join("editor0")).unwrap(), @r###"
    JJ: Enter a description for the first commit.

    JJ: This commit contains the following changes:
    JJ:     A file2

    JJ: Lines starting with "JJ: " (like this one) will be removed.
    "###);
    assert!(!test_env.env_root().join("editor1").exists());

    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r###"
    @  zsuskulnrvyr false
    ◉  qpvuntsmwlqt false
    ◉  zzzzzzzzzzzz true
    "###);

    // The author dates of the new commits should be inherited from the commit being
    // split. The committer dates should be newer.
    insta::assert_snapshot!(get_recorded_dates(&test_env, &repo_path,"@"), @r###"
    Author date:  2001-02-03 04:05:07.000 +07:00
    Committer date: 2001-02-03 04:05:10.000 +07:00
    "###);
    insta::assert_snapshot!(get_recorded_dates(&test_env, &repo_path,"@-"), @r###"
    Author date:  2001-02-03 04:05:07.000 +07:00
    Committer date: 2001-02-03 04:05:10.000 +07:00
    "###);

    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s", "-r", "@-"]);
    insta::assert_snapshot!(stdout, @r###"
    A file2
    "###);
    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s"]);
    insta::assert_snapshot!(stdout, @r###"
    A file1
    A file3
    "###);

    // Insert an empty commit after @- with "split ."
    test_env.set_up_fake_editor();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["split", "-r", "@-", "."]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Rebased 1 descendant commits
    First part: qpvuntsm b76d731d (no description set)
    Second part: znkkpsqq 924604b2 (empty) (no description set)
    Working copy now at: zsuskuln fffe30fb (no description set)
    Parent commit      : znkkpsqq 924604b2 (empty) (no description set)
    "###);

    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r###"
    @  zsuskulnrvyr false
    ◉  znkkpsqqskkl true
    ◉  qpvuntsmwlqt false
    ◉  zzzzzzzzzzzz true
    "###);

    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s", "-r", "@--"]);
    insta::assert_snapshot!(stdout, @r###"
    A file2
    "###);

    // Remove newly created empty commit
    test_env.jj_cmd_ok(&repo_path, &["abandon", "@-"]);

    // Insert an empty commit before @- with "split nonexistent"
    test_env.set_up_fake_editor();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["split", "-r", "@-", "nonexistent"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Warning: The given paths do not match any file: nonexistent
    Rebased 1 descendant commits
    First part: qpvuntsm 7086b0bc (empty) (no description set)
    Second part: lylxulpl 2252ed18 (no description set)
    Working copy now at: zsuskuln a3f2136a (no description set)
    Parent commit      : lylxulpl 2252ed18 (no description set)
    "###);

    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r###"
    @  zsuskulnrvyr false
    ◉  lylxulplsnyw false
    ◉  qpvuntsmwlqt true
    ◉  zzzzzzzzzzzz true
    "###);

    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s", "-r", "@-"]);
    insta::assert_snapshot!(stdout, @r###"
    A file2
    "###);
}

#[test]
fn test_split_with_non_empty_description() {
    let mut test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    test_env.add_config(r#"ui.default-description = "\n\nTESTED=TODO""#);
    let workspace_path = test_env.env_root().join("repo");

    std::fs::write(workspace_path.join("file1"), "foo\n").unwrap();
    std::fs::write(workspace_path.join("file2"), "bar\n").unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["describe", "-m", "test"]);
    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(
        edit_script,
        [
            "dump editor1",
            "write\npart 1",
            "next invocation\n",
            "dump editor2",
            "write\npart 2",
        ]
        .join("\0"),
    )
    .unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["split", "file1"]);

    assert_eq!(
        std::fs::read_to_string(test_env.env_root().join("editor1")).unwrap(),
        r#"JJ: Enter a description for the first commit.
test

JJ: This commit contains the following changes:
JJ:     A file1

JJ: Lines starting with "JJ: " (like this one) will be removed.
"#
    );
    assert_eq!(
        std::fs::read_to_string(test_env.env_root().join("editor2")).unwrap(),
        r#"JJ: Enter a description for the second commit.
test

JJ: This commit contains the following changes:
JJ:     A file2

JJ: Lines starting with "JJ: " (like this one) will be removed.
"#
    );
    insta::assert_snapshot!(get_log_output(&test_env, &workspace_path), @r###"
    @  kkmpptxzrspx false part 2
    ◉  qpvuntsmwlqt false part 1
    ◉  zzzzzzzzzzzz true
    "###);
}

#[test]
fn test_split_with_default_description() {
    let mut test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    test_env.add_config(r#"ui.default-description = "\n\nTESTED=TODO""#);
    let workspace_path = test_env.env_root().join("repo");

    std::fs::write(workspace_path.join("file1"), "foo\n").unwrap();
    std::fs::write(workspace_path.join("file2"), "bar\n").unwrap();

    // Create a branch pointing to the commit. It will be moved to the second
    // commit after the split.
    test_env.jj_cmd_ok(&workspace_path, &["branch", "create", "test_branch"]);

    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(
        edit_script,
        ["dump editor1", "next invocation\n", "dump editor2"].join("\0"),
    )
    .unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["split", "file1"]);

    // Since the commit being split has no description, the user will only be
    // prompted to add a description to the first commit, which will use the
    // default value we set. The second commit will inherit the empty
    // description from the commit being split.
    assert_eq!(
        std::fs::read_to_string(test_env.env_root().join("editor1")).unwrap(),
        r#"JJ: Enter a description for the first commit.


TESTED=TODO
JJ: This commit contains the following changes:
JJ:     A file1

JJ: Lines starting with "JJ: " (like this one) will be removed.
"#
    );
    assert!(!test_env.env_root().join("editor2").exists());
    insta::assert_snapshot!(get_log_output(&test_env, &workspace_path), @r###"
    @  kkmpptxzrspx false test_branch
    ◉  qpvuntsmwlqt false TESTED=TODO
    ◉  zzzzzzzzzzzz true
    "###);
}

#[test]
// Split a commit with no descendants into siblings. Also tests that the default
// description is set correctly on the first commit.
fn test_split_siblings_no_descendants() {
    let mut test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    test_env.add_config(r#"ui.default-description = "\n\nTESTED=TODO""#);
    let workspace_path = test_env.env_root().join("repo");

    std::fs::write(workspace_path.join("file1"), "foo\n").unwrap();
    std::fs::write(workspace_path.join("file2"), "bar\n").unwrap();

    // Create a branch pointing to the commit. It will be moved to the second
    // commit after the split.
    test_env.jj_cmd_ok(&workspace_path, &["branch", "create", "test_branch"]);
    insta::assert_snapshot!(get_log_output(&test_env, &workspace_path), @r###"
    @  qpvuntsmwlqt false test_branch
    ◉  zzzzzzzzzzzz true
    "###);

    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(
        edit_script,
        ["dump editor1", "next invocation\n", "dump editor2"].join("\0"),
    )
    .unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["split", "--siblings", "file1"]);

    // Since the commit being split has no description, the user will only be
    // prompted to add a description to the first commit, which will use the
    // default value we set. The second commit will inherit the empty
    // description from the commit being split.
    assert_eq!(
        std::fs::read_to_string(test_env.env_root().join("editor1")).unwrap(),
        r#"JJ: Enter a description for the first commit.


TESTED=TODO
JJ: This commit contains the following changes:
JJ:     A file1

JJ: Lines starting with "JJ: " (like this one) will be removed.
"#
    );
    assert!(!test_env.env_root().join("editor2").exists());
    insta::assert_snapshot!(get_log_output(&test_env, &workspace_path), @r###"
    @  zsuskulnrvyr false test_branch
    │ ◉  qpvuntsmwlqt false TESTED=TODO
    ├─╯
    ◉  zzzzzzzzzzzz true
    "###);
}

#[test]
fn test_split_siblings_with_descendants() {
    // Configure the environment and make the initial commits.
    let mut test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    // test_env.add_config(r#"ui.default-description = "\n\nTESTED=TODO""#);
    let workspace_path = test_env.env_root().join("repo");

    // First commit. This is the one we will split later.
    std::fs::write(workspace_path.join("file1"), "foo\n").unwrap();
    std::fs::write(workspace_path.join("file2"), "bar\n").unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["commit", "-m", "Add file1 & file2"]);
    // Second commit. This will be the child of the sibling commits after the split.
    std::fs::write(workspace_path.join("file3"), "baz\n").unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["commit", "-m", "Add file3"]);
    // Third commit.
    std::fs::write(workspace_path.join("file4"), "foobarbaz\n").unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["describe", "-m", "Add file4"]);
    // Move back to the previous commit so that we don't have to pass a revision
    // to the split command.
    test_env.jj_cmd_ok(&workspace_path, &["prev", "--edit"]);
    test_env.jj_cmd_ok(&workspace_path, &["prev", "--edit"]);

    // Set up the editor and do the split.
    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(
        edit_script,
        [
            "dump editor1",
            "write\nAdd file1",
            "next invocation\n",
            "dump editor2",
            "write\nAdd file2",
        ]
        .join("\0"),
    )
    .unwrap();
    test_env.jj_cmd_ok(&workspace_path, &["split", "--siblings", "file1"]);

    // The commit we're splitting has a description, so the user will be
    // prompted to enter a description for each of the sibling commits.
    assert_eq!(
        std::fs::read_to_string(test_env.env_root().join("editor1")).unwrap(),
        r#"JJ: Enter a description for the first commit.
Add file1 & file2

JJ: This commit contains the following changes:
JJ:     A file1

JJ: Lines starting with "JJ: " (like this one) will be removed.
"#
    );
    assert_eq!(
        std::fs::read_to_string(test_env.env_root().join("editor2")).unwrap(),
        r#"JJ: Enter a description for the second commit.
Add file1 & file2

JJ: This commit contains the following changes:
JJ:     A file2

JJ: Lines starting with "JJ: " (like this one) will be removed.
"#
    );

    insta::assert_snapshot!(get_log_output(&test_env, &workspace_path), @r###"
    ◉  kkmpptxzrspx false Add file4
    ◉    rlvkpnrzqnoo false Add file3
    ├─╮
    │ @  yqosqzytrlsw false Add file2
    ◉ │  qpvuntsmwlqt false Add file1
    ├─╯
    ◉  zzzzzzzzzzzz true
    "###);
}
