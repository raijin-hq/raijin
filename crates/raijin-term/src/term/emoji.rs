//! Emoji Presentation property detection.
//!
//! Unicode Technical Report #51 defines the Emoji_Presentation property for
//! characters that should be displayed as emoji by default (without needing
//! the U+FE0F variation selector). Terminals must render these as 2 cells wide.
//!
//! UAX#11 (East Asian Width) classifies many of these as "Neutral" (width 1),
//! so we need this override for correct terminal display.
//!
//! Source: https://unicode.org/Public/16.0.0/ucd/emoji/emoji-data.txt
//! Property: Emoji_Presentation=Yes

/// Returns `true` if the character has the Emoji_Presentation property,
/// meaning it should be displayed as a 2-cell-wide emoji by default.
#[inline]
pub fn emoji_presentation(c: char) -> bool {
    let cp = c as u32;

    // Fast path: most text is ASCII/Latin
    if cp < 0x2000 {
        return false;
    }

    // BMP Emoji_Presentation=Yes characters (curated from emoji-data.txt)
    match cp {
        0x231A..=0x231B => true, // ⌚⌛
        0x23E9..=0x23F3 => true, // ⏩⏪⏫⏬⏭⏮⏯⏰⏱⏲⏳
        0x23F8..=0x23FA => true, // ⏸⏹⏺
        0x25FD..=0x25FE => true, // ◽◾
        0x2614..=0x2615 => true, // ☔☕
        0x2648..=0x2653 => true, // ♈♉♊♋♌♍♎♏♐♑♒♓
        0x267F          => true, // ♿
        0x2693          => true, // ⚓
        0x26A1          => true, // ⚡
        0x26AA..=0x26AB => true, // ⚪⚫
        0x26BD..=0x26BE => true, // ⚽⚾
        0x26C4..=0x26C5 => true, // ⛄⛅
        0x26CE          => true, // ⛎
        0x26D4          => true, // ⛔
        0x26EA          => true, // ⛪
        0x26F2..=0x26F3 => true, // ⛲⛳
        0x26F5          => true, // ⛵
        0x26FA          => true, // ⛺
        0x26FD          => true, // ⛽
        0x2702          => true, // ✂
        0x2705          => true, // ✅
        0x2708..=0x270D => true, // ✈✉✊✋✌✍
        0x270F          => true, // ✏
        0x2712          => true, // ✒
        0x2714          => true, // ✔
        0x2716          => true, // ✖
        0x271D          => true, // ✝
        0x2721          => true, // ✡
        0x2728          => true, // ✨
        0x2733..=0x2734 => true, // ✳✴
        0x2744          => true, // ❄
        0x2747          => true, // ❇
        0x274C          => true, // ❌
        0x274E          => true, // ❎
        0x2753..=0x2755 => true, // ❓❔❕
        0x2757          => true, // ❗
        0x2763..=0x2764 => true, // ❣❤
        0x2795..=0x2797 => true, // ➕➖➗
        0x27A1          => true, // ➡
        0x27B0          => true, // ➰
        0x27BF          => true, // ➿
        0x2934..=0x2935 => true, // ⤴⤵
        0x2B05..=0x2B07 => true, // ⬅⬆⬇
        0x2B1B..=0x2B1C => true, // ⬛⬜
        0x2B50          => true, // ⭐
        0x2B55          => true, // ⭕
        0x3030          => true, // 〰
        0x303D          => true, // 〽
        0x3297          => true, // ㊗
        0x3299          => true, // ㊙

        // Supplementary Plane emoji (U+1F000+)
        // Most are already width 2 via UAX#11, but some slip through.
        0x1F004         => true, // 🀄
        0x1F0CF         => true, // 🃏
        0x1F170..=0x1F171 => true, // 🅰🅱
        0x1F17E..=0x1F17F => true, // 🅾🅿
        0x1F18E         => true, // 🆎
        0x1F191..=0x1F19A => true, // 🆑..🆚
        0x1F1E6..=0x1F1FF => true, // 🇦..🇿 (regional indicators)
        0x1F201..=0x1F202 => true, // 🈁🈂
        0x1F21A         => true, // 🈚
        0x1F22F         => true, // 🈯
        0x1F232..=0x1F23A => true, // 🈲..🈺
        0x1F250..=0x1F251 => true, // 🉐🉑
        0x1F300..=0x1F9FF => true, // Misc Symbols, Emoticons, Transport, Supplemental
        0x1FA00..=0x1FA6F => true, // Chess, Extended-A
        0x1FA70..=0x1FAFF => true, // Extended-A continued
        0x1FB00..=0x1FBFF => true, // Symbols for Legacy Computing (some)

        _ => false,
    }
}
