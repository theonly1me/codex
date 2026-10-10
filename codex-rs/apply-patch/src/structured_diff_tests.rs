use crate::structured_file_diff;

#[test]
fn structured_diff_keeps_context_and_reports_the_actual_changed_line() {
    let original = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\n";
    let updated = "one\ntwo\nthree\nfour\nfive\nsix\nchanged\neight\nnine\n";
    let hunks = structured_file_diff(original, updated);
    assert_eq!(hunks.len(), 1);
    let hunk = &hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (4, 4));
    assert_eq!((hunk.old_lines, hunk.new_lines), (6, 6));
    assert_eq!(
        hunk.lines,
        [
            " four", " five", " six", "-seven", "+changed", " eight", " nine"
        ]
    );
}
