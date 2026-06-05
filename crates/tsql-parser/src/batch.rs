//! GO batch splitter for T-SQL.
//!
//! T-SQL uses GO as a batch separator. GO must appear at the start of a line
//! (with optional leading whitespace). This module splits a multi-batch T-SQL
//! input into individual batches.

/// Split T-SQL input on GO batch separators.
///
/// GO must appear at the start of a line (with optional leading whitespace).
/// Respects string literals and comments — GO inside a string or comment
/// is not treated as a batch separator.
pub fn split_batches(input: &str) -> Vec<String> {
    let mut batches = Vec::new();
    let mut current_batch = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < chars.len() {
        let ch = chars[i];

        // Handle string literals
        if !in_line_comment && !in_block_comment && ch == '\'' {
            in_string = !in_string;
            current_batch.push(ch);
            i += 1;
            continue;
        }

        if in_string {
            current_batch.push(ch);
            i += 1;
            continue;
        }

        // Handle line comments
        if !in_block_comment && ch == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            in_line_comment = true;
            current_batch.push(ch);
            i += 1;
            continue;
        }

        if in_line_comment {
            current_batch.push(ch);
            if ch == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }

        // Handle block comments
        if !in_line_comment && ch == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            in_block_comment = true;
            current_batch.push(ch);
            i += 1;
            continue;
        }

        if in_block_comment {
            current_batch.push(ch);
            if ch == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                current_batch.push('/');
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        // Check for GO at start of line
        if ch == '\n' || (i == 0 && (ch == 'G' || ch == 'g')) {
            if ch == '\n' {
                current_batch.push(ch);
                i += 1;
                // Check if next non-whitespace is GO
                let _start = i;
                while i < chars.len() && chars[i].is_whitespace() && chars[i] != '\n' {
                    current_batch.push(chars[i]);
                    i += 1;
                }
                if i + 1 < chars.len()
                    && (chars[i] == 'G' || chars[i] == 'g')
                    && (chars[i + 1] == 'O' || chars[i + 1] == 'o')
                {
                    // Check it's a complete GO (next char is whitespace or end)
                    let after_go = i + 2;
                    if after_go >= chars.len()
                        || chars[after_go].is_whitespace()
                        || chars[after_go] == '-'
                    {
                        // Consume GO and rest of line
                        i = after_go;
                        while i < chars.len() && chars[i] != '\n' {
                            i += 1;
                        }
                        // End current batch
                        let trimmed = current_batch.trim().to_string();
                        if !trimmed.is_empty() {
                            batches.push(trimmed);
                        }
                        current_batch.clear();
                        continue;
                    }
                }
                // Not GO, keep going
                continue;
            }
        }

        current_batch.push(ch);
        i += 1;
    }

    // Push last batch
    let trimmed = current_batch.trim().to_string();
    if !trimmed.is_empty() {
        batches.push(trimmed);
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_batch() {
        let input = "SELECT 1";
        let batches = split_batches(input);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0], "SELECT 1");
    }

    #[test]
    fn test_two_batches() {
        let input = "SELECT 1\nGO\nSELECT 2";
        let batches = split_batches(input);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0], "SELECT 1");
        assert_eq!(batches[1], "SELECT 2");
    }

    #[test]
    fn test_go_in_string() {
        let input = "SELECT 'GO is not a separator'";
        let batches = split_batches(input);
        assert_eq!(batches.len(), 1);
    }

    #[test]
    fn test_go_in_comment() {
        let input = "SELECT 1 -- GO is not a separator\nSELECT 2";
        let batches = split_batches(input);
        assert_eq!(batches.len(), 1);
    }

    #[test]
    fn test_multiple_batches() {
        let input = "CREATE TABLE t1 (id INT)\nGO\nINSERT INTO t1 VALUES (1)\nGO\nSELECT * FROM t1";
        let batches = split_batches(input);
        assert_eq!(batches.len(), 3);
    }
}
