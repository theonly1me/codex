use similar::ChangeTag;
use similar::TextDiff;

pub struct FileDiffHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub lines: Vec<String>,
}

pub fn structured_file_diff(original: &str, updated: &str) -> Vec<FileDiffHunk> {
    let diff = TextDiff::from_lines(original, updated);
    diff.grouped_ops(3)
        .into_iter()
        .filter_map(|group| {
            let first = group.first()?;
            let last = group.last()?;
            let old_start = first.old_range().start;
            let new_start = first.new_range().start;
            let lines = group
                .iter()
                .flat_map(|operation| diff.iter_changes(operation))
                .map(|change| {
                    let marker = match change.tag() {
                        ChangeTag::Equal => ' ',
                        ChangeTag::Delete => '-',
                        ChangeTag::Insert => '+',
                    };
                    format!("{marker}{}", change.value().trim_end_matches(['\r', '\n']))
                })
                .collect();
            Some(FileDiffHunk {
                old_start: old_start + 1,
                old_lines: last.old_range().end - old_start,
                new_start: new_start + 1,
                new_lines: last.new_range().end - new_start,
                lines,
            })
        })
        .collect()
}
