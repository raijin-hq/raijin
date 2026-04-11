/// Command correction system for typo detection.
///
/// When a command exits with code 127 (command not found), suggests corrections
/// using a combination of a static typo map and Damerau-Levenshtein distance
/// against known executables.
use strsim::damerau_levenshtein;

/// Common typo → correct command mappings for instant high-confidence corrections.
const TYPO_MAP: &[(&str, &str)] = &[
    ("gti", "git"),
    ("gi", "git"),
    ("sl", "ls"),
    ("dc", "cd"),
    ("grpe", "grep"),
    ("gerp", "grep"),
    ("dokcer", "docker"),
    ("dcoker", "docker"),
    ("pytohn", "python"),
    ("pyhton", "python"),
    ("ndoe", "node"),
    ("noed", "node"),
    ("claer", "clear"),
    ("clea", "clear"),
    ("eixt", "exit"),
    ("exti", "exit"),
    ("whcih", "which"),
    ("wihch", "which"),
    ("suod", "sudo"),
    ("sduo", "sudo"),
    ("mkae", "make"),
    ("maek", "make"),
    ("carg", "cargo"),
    ("crago", "cargo"),
    ("cagro", "cargo"),
    ("brwe", "brew"),
    ("bew", "brew"),
    ("nivm", "nvim"),
    ("nvi", "nvim"),
    ("vmi", "vim"),
    ("got", "go"),
    ("cd..", "cd .."),
    ("ls-la", "ls -la"),
    ("gits", "git status"),
    ("gitp", "git push"),
    ("gitl", "git log"),
];

/// Result of a correction suggestion.
#[derive(Debug, Clone)]
pub struct CorrectionResult {
    pub original: String,
    pub suggestion: String,
    pub confidence: f64,
}

/// Suggest a correction for a failed command.
///
/// Only triggers on exit code 127 (command not found).
/// Returns `None` if no good correction is found.
pub fn suggest_correction(
    command_line: &str,
    exit_code: i32,
    known_commands: &[String],
) -> Option<CorrectionResult> {
    if exit_code != 127 {
        return None;
    }

    let trimmed = command_line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first_word = trimmed.split_whitespace().next()?;
    let rest = &trimmed[first_word.len()..];

    // 1. Check typo map (instant, high confidence)
    if let Some(&(_, correct)) = TYPO_MAP.iter().find(|(typo, _)| *typo == first_word) {
        return Some(CorrectionResult {
            original: command_line.to_string(),
            suggestion: format!("{}{}", correct, rest),
            confidence: 1.0,
        });
    }

    // 2. Damerau-Levenshtein against known commands (max distance 2)
    if first_word.len() < 2 {
        return None;
    }

    let mut best: Option<(&str, usize)> = None;
    for cmd in known_commands {
        let dist = damerau_levenshtein(first_word, cmd);
        if dist > 0 && dist <= 2 && dist < best.map_or(usize::MAX, |b| b.1) {
            best = Some((cmd, dist));
        }
    }

    best.map(|(cmd, dist)| CorrectionResult {
        original: command_line.to_string(),
        suggestion: format!("{}{}", cmd, rest),
        confidence: 1.0 - (dist as f64 / first_word.len().max(1) as f64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typo_map() {
        let result = suggest_correction("gti status", 127, &[]);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.suggestion, "git status");
        assert_eq!(r.confidence, 1.0);
    }

    #[test]
    fn test_levenshtein() {
        let known = vec!["git".to_string(), "grep".to_string(), "go".to_string()];
        let result = suggest_correction("gitt status", 127, &known);
        assert!(result.is_some());
        assert_eq!(result.unwrap().suggestion, "git status");
    }

    #[test]
    fn test_no_correction_on_success() {
        let result = suggest_correction("ls", 0, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_no_correction_for_unknown() {
        let known = vec!["git".to_string()];
        let result = suggest_correction("zzzzzzz", 127, &known);
        assert!(result.is_none()); // Distance > 2
    }

    #[test]
    fn test_exit_code_127_only() {
        let result = suggest_correction("gti", 1, &[]);
        assert!(result.is_none());
    }
}
