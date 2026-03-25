//! Pure utility functions for text measurement and cursor navigation.

pub(crate) fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.saturating_sub(1);
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

pub(crate) fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos + 1;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

/// Build a flat list of visual rows as (line_start_byte, line_len) accounting for wrapping.
pub(crate) fn visual_rows(text: &str, wrap_width: usize) -> Vec<(usize, usize)> {
    let mut rows = Vec::new();
    let mut offset: usize = 0;
    for line in text.split('\n') {
        if line.is_empty() {
            rows.push((offset, 0));
        } else {
            let mut remaining = line.len();
            let mut pos = 0;
            while remaining > 0 {
                let chunk = remaining.min(wrap_width);
                rows.push((offset + pos, chunk));
                pos += chunk;
                remaining -= chunk;
            }
        }
        offset += line.len() + 1; // +1 for the \n
    }
    rows
}

pub(crate) fn byte_to_row_col_with(rows: &[(usize, usize)], byte_pos: usize) -> (usize, usize) {
    for (i, &(start, len)) in rows.iter().enumerate() {
        let end = start + len;
        // Cursor can be at end of row (after last char) only if it's the last row
        // or if the next row starts a new logical line
        if byte_pos >= start && byte_pos <= end {
            // If exactly at end and there's a next row that continues this wrap, go to next row
            if byte_pos == end && i + 1 < rows.len() {
                let next_start = rows[i + 1].0;
                if next_start == end {
                    // next row is a continuation of the same logical line
                    return (i + 1, 0);
                }
            }
            return (i, byte_pos - start);
        }
    }
    // Past end: last row
    let last = rows.len().saturating_sub(1);
    let col = if let Some(&(start, _)) = rows.last() {
        byte_pos.saturating_sub(start)
    } else {
        0
    };
    (last, col)
}

pub(crate) fn task_visual_height(task: &crate::model::Task, title_width: usize) -> usize {
    let title_lines = wrapped_line_count(&task.title, title_width);
    let tag_lines = if task.tags.is_empty() { 0 } else { 1 };
    title_lines + tag_lines + 1 // +1 for due date line
}

pub(crate) fn wrapped_line_count(text: &str, width: usize) -> usize {
    if width == 0 || text.is_empty() {
        return 1;
    }
    let mut count = 0;
    let mut remaining = text;
    while !remaining.is_empty() {
        count += 1;
        if remaining.len() <= width {
            break;
        }
        let boundary = width.min(remaining.len());
        let split = if let Some(space_pos) = remaining[..boundary].rfind(' ') {
            space_pos + 1
        } else {
            boundary
        };
        remaining = &remaining[split..];
    }
    count
}

pub(crate) fn row_col_to_byte_with(
    rows: &[(usize, usize)],
    row: usize,
    col: usize,
    text_len: usize,
) -> usize {
    if let Some(&(start, len)) = rows.get(row) {
        let clamped_col = col.min(len);
        (start + clamped_col).min(text_len)
    } else {
        text_len
    }
}
