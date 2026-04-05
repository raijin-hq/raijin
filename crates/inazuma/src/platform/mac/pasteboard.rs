use std::path::PathBuf;

use objc2_app_kit::{
    NSPasteboard, NSPasteboardNameFind, NSPasteboardType, NSPasteboardTypePNG,
    NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSArray, NSData, NSString};
use objc2::rc::Retained;
use smallvec::SmallVec;
use strum::IntoEnumIterator as _;

use inazuma::{
    ClipboardEntry, ClipboardItem, ClipboardString, ExternalPaths, Image, ImageFormat, hash,
};

#[allow(deprecated)]
use objc2_app_kit::NSFilenamesPboardType;

pub struct Pasteboard {
    inner: Retained<NSPasteboard>,
    text_hash_type: Retained<NSString>,
    metadata_type: Retained<NSString>,
}

impl Pasteboard {
    pub fn general() -> Self {
        Self::new(NSPasteboard::generalPasteboard())
    }

    pub fn find() -> Self {
        // SAFETY: NSPasteboardNameFind is a valid extern static from AppKit.
        // Rust 2024 requires unsafe for extern static access (RFC 3484).
        Self::new(unsafe { NSPasteboard::pasteboardWithName(NSPasteboardNameFind) })
    }

    #[cfg(test)]
    pub fn unique() -> Self {
        Self::new(NSPasteboard::pasteboardWithUniqueName())
    }

    fn new(inner: Retained<NSPasteboard>) -> Self {
        Self {
            inner,
            text_hash_type: NSString::from_str("zed-text-hash"),
            metadata_type: NSString::from_str("zed-metadata"),
        }
    }

    #[allow(deprecated)]
    pub fn read(&self) -> Option<ClipboardItem> {
        // Check for file paths first
        let filenames_type: &NSPasteboardType = unsafe { NSFilenamesPboardType };
        if let Some(plist) = self.inner.propertyListForType(filenames_type) {
            // The property list for NSFilenamesPboardType is an NSArray of NSString
            let array: &NSArray<NSString> =
                unsafe { &*((&*plist as *const objc2::runtime::AnyObject).cast()) };
            if array.count() > 0 {
                let mut paths = SmallVec::new();
                for file in unsafe { array.iter_unchecked() } {
                    paths.push(PathBuf::from(file.to_string()));
                }
                if !paths.is_empty() {
                    let mut entries = vec![ClipboardEntry::ExternalPaths(ExternalPaths(paths))];

                    // Also include the string representation so text editors can
                    // paste the path as text.
                    if let Some(string_item) = self.read_string_from_pasteboard() {
                        entries.push(string_item);
                    }

                    return Some(ClipboardItem { entries });
                }
            }
        }

        // Next, check for a plain string.
        if let Some(string_entry) = self.read_string_from_pasteboard() {
            return Some(ClipboardItem {
                entries: vec![string_entry],
            });
        }

        // Finally, try the various supported image types.
        for format in ImageFormat::iter() {
            if let Some(item) = self.read_image(format) {
                return Some(item);
            }
        }

        None
    }

    fn read_image(&self, format: ImageFormat) -> Option<ClipboardItem> {
        let ut_type: PasteboardUTType = format.into();

        let types = self.inner.types()?;
        if types.containsObject(&ut_type.0) {
            self.data_for_type(&ut_type.0).map(|bytes| {
                let bytes = bytes.to_vec();
                let id = hash(&bytes);

                ClipboardItem {
                    entries: vec![ClipboardEntry::Image(Image { format, bytes, id })],
                }
            })
        } else {
            None
        }
    }

    fn read_string_from_pasteboard(&self) -> Option<ClipboardEntry> {
        let string_type = NSString::from_str("public.utf8-plain-text");

        let types = self.inner.types()?;
        if !types.containsObject(&string_type) {
            return None;
        }

        let data = self.inner.dataForType(&string_type)?;
        let text_bytes = unsafe { data.as_bytes_unchecked() };
        let text = String::from_utf8_lossy(text_bytes).to_string();

        let metadata = self
            .data_for_type(&self.text_hash_type)
            .and_then(|hash_bytes| {
                let hash_bytes: [u8; 8] = hash_bytes.try_into().ok()?;
                let hash = u64::from_be_bytes(hash_bytes);
                let metadata = self.data_for_type(&self.metadata_type)?;

                if hash == ClipboardString::text_hash(&text) {
                    String::from_utf8(metadata.to_vec()).ok()
                } else {
                    None
                }
            });

        Some(ClipboardEntry::String(ClipboardString { text, metadata }))
    }

    fn data_for_type(&self, kind: &NSString) -> Option<Vec<u8>> {
        let data = self.inner.dataForType(kind)?;
        Some(data.to_vec())
    }

    pub fn write(&self, item: ClipboardItem) {
        match item.entries.as_slice() {
            [] => {
                // Writing an empty list of entries just clears the clipboard.
                self.inner.clearContents();
            }
            [ClipboardEntry::String(string)] => {
                self.write_plaintext(string);
            }
            [ClipboardEntry::Image(image)] => {
                self.write_image(image);
            }
            [ClipboardEntry::ExternalPaths(_)] => {}
            _ => {
                let mut combined = ClipboardString {
                    text: String::new(),
                    metadata: None,
                };

                for entry in item.entries {
                    match entry {
                        ClipboardEntry::String(text) => {
                            combined.text.push_str(&text.text());
                            if combined.metadata.is_none() {
                                combined.metadata = text.metadata;
                            }
                        }
                        _ => {}
                    }
                }

                self.write_plaintext(&combined);
            }
        }
    }

    fn write_plaintext(&self, string: &ClipboardString) {
        self.inner.clearContents();

        let text_data = NSData::with_bytes(string.text.as_bytes());
        self.inner
            .setData_forType(Some(&text_data), unsafe { NSPasteboardTypeString });

        if let Some(metadata) = string.metadata.as_ref() {
            let hash_bytes = ClipboardString::text_hash(&string.text).to_be_bytes();
            let hash_data = NSData::with_bytes(&hash_bytes);
            self.inner
                .setData_forType(Some(&hash_data), &self.text_hash_type);

            let metadata_data = NSData::with_bytes(metadata.as_bytes());
            self.inner
                .setData_forType(Some(&metadata_data), &self.metadata_type);
        }
    }

    fn write_image(&self, image: &Image) {
        self.inner.clearContents();

        let data = NSData::with_bytes(&image.bytes);
        let ut_type: PasteboardUTType = image.format.into();
        self.inner.setData_forType(Some(&data), &ut_type.0);
    }
}

impl From<ImageFormat> for PasteboardUTType {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::Png => Self::png(),
            ImageFormat::Jpeg => Self::jpeg(),
            ImageFormat::Tiff => Self::tiff(),
            ImageFormat::Webp => Self::webp(),
            ImageFormat::Gif => Self::gif(),
            ImageFormat::Bmp => Self::bmp(),
            ImageFormat::Svg => Self::svg(),
            ImageFormat::Ico => Self::ico(),
        }
    }
}

/// Wrapper around UTType identifiers as NSString for pasteboard operations.
/// See https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/
pub struct PasteboardUTType(Retained<NSString>);

impl PasteboardUTType {
    pub fn png() -> Self {
        Self(unsafe { NSPasteboardTypePNG }.to_owned())
    }

    pub fn jpeg() -> Self {
        Self(NSString::from_str("public.jpeg"))
    }

    pub fn gif() -> Self {
        Self(NSString::from_str("com.compuserve.gif"))
    }

    pub fn webp() -> Self {
        Self(NSString::from_str("org.webmproject.webp"))
    }

    pub fn bmp() -> Self {
        Self(NSString::from_str("com.microsoft.bmp"))
    }

    pub fn svg() -> Self {
        Self(NSString::from_str("public.svg-image"))
    }

    pub fn ico() -> Self {
        Self(NSString::from_str("com.microsoft.ico"))
    }

    pub fn tiff() -> Self {
        Self(unsafe { NSPasteboardTypeTIFF }.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use objc2_app_kit::{
        NSPasteboardTypePNG, NSPasteboardTypeString,
    };
    use objc2_foundation::{NSArray, NSData, NSString};

    use inazuma::{ClipboardEntry, ClipboardItem, ClipboardString, ImageFormat};

    use super::*;

    /// macOS pasteboard operations can race even with unique pasteboards.
    /// Serialize all pasteboard tests to avoid flaky failures.
    static PASTEBOARD_LOCK: Mutex<()> = Mutex::new(());

    #[allow(deprecated)]
    unsafe fn simulate_external_file_copy(pasteboard: &Pasteboard, paths: &[&str]) {
        let ns_paths: Vec<Retained<NSString>> =
            paths.iter().map(|p| NSString::from_str(p)).collect();
        let ns_array = NSArray::from_retained_slice(&ns_paths);

        let filenames_type: &NSPasteboardType = unsafe { NSFilenamesPboardType };
        let string_type: &NSPasteboardType = NSPasteboardTypeString;
        let types_array =
            NSArray::from_slice(&[filenames_type, string_type]);

        unsafe {
            pasteboard
                .inner
                .declareTypes_owner(&types_array, None);

            pasteboard
                .inner
                .setPropertyList_forType(&ns_array, filenames_type);
        }

        let joined = paths.join("\n");
        let data = NSData::with_bytes(joined.as_bytes());
        pasteboard
            .inner
            .setData_forType(Some(&data), string_type);
    }

    #[test]
    fn test_string() {
        let _lock = PASTEBOARD_LOCK.lock().unwrap();
        let pasteboard = Pasteboard::unique();
        assert_eq!(pasteboard.read(), None);

        let item = ClipboardItem::new_string("1".to_string());
        pasteboard.write(item.clone());
        assert_eq!(pasteboard.read(), Some(item));

        let item = ClipboardItem {
            entries: vec![ClipboardEntry::String(
                ClipboardString::new("2".to_string()).with_json_metadata(vec![3, 4]),
            )],
        };
        pasteboard.write(item.clone());
        assert_eq!(pasteboard.read(), Some(item));

        let text_from_other_app = "text from other app";
        let data = NSData::with_bytes(text_from_other_app.as_bytes());
        pasteboard
            .inner
            .setData_forType(Some(&data), unsafe { NSPasteboardTypeString });

        assert_eq!(
            pasteboard.read(),
            Some(ClipboardItem::new_string(text_from_other_app.to_string()))
        );
    }

    #[test]
    fn test_read_external_path() {
        let _lock = PASTEBOARD_LOCK.lock().unwrap();
        let pasteboard = Pasteboard::unique();

        unsafe {
            simulate_external_file_copy(&pasteboard, &["/test.txt"]);
        }

        let item = pasteboard.read().expect("should read clipboard item");

        // Test both ExternalPaths and String entries exist
        assert_eq!(item.entries.len(), 2);

        // Test first entry is ExternalPaths
        match &item.entries[0] {
            ClipboardEntry::ExternalPaths(ep) => {
                assert_eq!(ep.paths(), &[PathBuf::from("/test.txt")]);
            }
            other => panic!("expected ExternalPaths, got {:?}", other),
        }

        // Test second entry is String
        match &item.entries[1] {
            ClipboardEntry::String(s) => {
                assert_eq!(s.text(), "/test.txt");
            }
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_read_external_paths_with_spaces() {
        let _lock = PASTEBOARD_LOCK.lock().unwrap();
        let pasteboard = Pasteboard::unique();
        let paths = ["/some file with spaces.txt"];

        unsafe {
            simulate_external_file_copy(&pasteboard, &paths);
        }

        let item = pasteboard.read().expect("should read clipboard item");

        match &item.entries[0] {
            ClipboardEntry::ExternalPaths(ep) => {
                assert_eq!(ep.paths(), &[PathBuf::from("/some file with spaces.txt")]);
            }
            other => panic!("expected ExternalPaths, got {:?}", other),
        }
    }

    #[test]
    fn test_read_multiple_external_paths() {
        let _lock = PASTEBOARD_LOCK.lock().unwrap();
        let pasteboard = Pasteboard::unique();
        let paths = ["/file.txt", "/image.png"];

        unsafe {
            simulate_external_file_copy(&pasteboard, &paths);
        }

        let item = pasteboard.read().expect("should read clipboard item");
        assert_eq!(item.entries.len(), 2);

        // Test both ExternalPaths and String entries exist
        match &item.entries[0] {
            ClipboardEntry::ExternalPaths(ep) => {
                assert_eq!(
                    ep.paths(),
                    &[PathBuf::from("/file.txt"), PathBuf::from("/image.png"),]
                );
            }
            other => panic!("expected ExternalPaths, got {:?}", other),
        }

        match &item.entries[1] {
            ClipboardEntry::String(s) => {
                assert_eq!(s.text(), "/file.txt\n/image.png");
                assert_eq!(s.metadata, None);
            }
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_read_image() {
        let _lock = PASTEBOARD_LOCK.lock().unwrap();
        let pasteboard = Pasteboard::unique();

        // Smallest valid PNG: 1x1 transparent pixel
        let png_bytes: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x62, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00,
            0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        let ns_png_type: &NSPasteboardType = unsafe { NSPasteboardTypePNG };
        let types_array = NSArray::from_slice(&[ns_png_type]);
        unsafe {
            pasteboard
                .inner
                .declareTypes_owner(&types_array, None);
        }

        let data = NSData::with_bytes(png_bytes);
        pasteboard.inner.setData_forType(Some(&data), ns_png_type);

        let item = pasteboard.read().expect("should read PNG image");

        // Test Image entry exists
        assert_eq!(item.entries.len(), 1);
        match &item.entries[0] {
            ClipboardEntry::Image(img) => {
                assert_eq!(img.format, ImageFormat::Png);
                assert_eq!(img.bytes, png_bytes);
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }
}
