use inazuma::{FontFallbacks, FontFeatures};
use objc2_core_foundation::{
    CFDictionary, CFIndex, CFMutableArray, CFNumber, CFNumberType, CFRetained, CFString,
    kCFTypeArrayCallBacks, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use objc2_core_text::{
    CTFont, CTFontDescriptor, kCTFontCascadeListAttribute, kCTFontFeatureSettingsAttribute,
    kCTFontOpenTypeFeatureTag, kCTFontOpenTypeFeatureValue, kCTFontURLAttribute,
};
use std::ffi::c_void;
use std::ptr;

/// Apply OpenType features and font fallbacks to a CTFont, returning the new CTFont.
///
/// This creates a new font descriptor with the specified features and fallback cascade,
/// then copies the font with the new descriptor applied.
pub fn apply_features_and_fallbacks(
    font: &CTFont,
    features: &FontFeatures,
    fallbacks: Option<&FontFallbacks>,
) -> anyhow::Result<CFRetained<CTFont>> {
    unsafe {
        let feature_array = generate_feature_array(features);

        let mut keys: Vec<*const c_void> =
            vec![kCTFontFeatureSettingsAttribute as *const _ as *const c_void];
        let mut values: Vec<*const c_void> =
            vec![&*feature_array as *const _ as *const c_void];

        let fallback_array;
        if let Some(fallbacks) = fallbacks
            && !fallbacks.fallback_list().is_empty()
        {
            fallback_array = generate_fallback_array(fallbacks, font);
            keys.push(kCTFontCascadeListAttribute as *const _ as *const c_void);
            values.push(&*fallback_array as *const _ as *const c_void);
        }

        let attrs = CFDictionary::new(
            None,
            keys.as_mut_ptr(),
            values.as_mut_ptr(),
            keys.len() as CFIndex,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
        .expect("failed to create dictionary");

        let new_descriptor = CTFontDescriptor::with_attributes(&attrs);

        let new_font = font.copy_with_attributes(
            0.0,
            ptr::null(),
            Some(&new_descriptor),
        );

        Ok(new_font)
    }
}

fn generate_feature_array(features: &FontFeatures) -> CFRetained<CFMutableArray> {
    unsafe {
        let feature_array =
            CFMutableArray::new(None, 0, &kCFTypeArrayCallBacks).expect("failed to create array");
        for (tag, value) in features.tag_value_list() {
            let tag_str = CFString::from_str(tag);
            let val = *value as i32;
            let value_num =
                CFNumber::new(None, CFNumberType::SInt32Type, &val as *const _ as *const c_void)
                    .expect("failed to create CFNumber");

            let mut keys: [*const c_void; 2] = [
                kCTFontOpenTypeFeatureTag as *const _ as *const c_void,
                kCTFontOpenTypeFeatureValue as *const _ as *const c_void,
            ];
            let mut values: [*const c_void; 2] = [
                &*tag_str as *const _ as *const c_void,
                &*value_num as *const _ as *const c_void,
            ];
            let dict = CFDictionary::new(
                None,
                keys.as_mut_ptr(),
                values.as_mut_ptr(),
                2,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
            .expect("failed to create feature dict");
            CFMutableArray::append_value(
                Some(&feature_array),
                &*dict as *const _ as *const c_void,
            );
        }
        feature_array
    }
}

fn generate_fallback_array(
    fallbacks: &FontFallbacks,
    font: &CTFont,
) -> CFRetained<CFMutableArray> {
    unsafe {
        let fallback_array =
            CFMutableArray::new(None, 0, &kCFTypeArrayCallBacks).expect("failed to create array");
        for user_fallback in fallbacks.fallback_list() {
            let name = CFString::from_str(user_fallback.as_str());
            let fallback_desc = CTFontDescriptor::with_name_and_size(&name, 0.0);
            CFMutableArray::append_value(
                Some(&fallback_array),
                &*fallback_desc as *const _ as *const c_void,
            );
        }
        append_system_fallbacks(&fallback_array, font);
        fallback_array
    }
}

fn append_system_fallbacks(fallback_array: &CFMutableArray, font: &CTFont) {
    unsafe {
        let preferred_languages = objc2_core_foundation::CFLocale::preferred_languages();
        let preferred_languages = preferred_languages.as_deref();

        let default_fallbacks = font.default_cascade_list_for_languages(preferred_languages);

        if let Some(default_fallbacks) = default_fallbacks {
            let count = default_fallbacks.count();
            for i in 0..count {
                let ptr = default_fallbacks.value_at_index(i);
                if ptr.is_null() {
                    continue;
                }
                let desc: &CTFontDescriptor =
                    &*(ptr as *const CTFontDescriptor);
                if desc.attribute(kCTFontURLAttribute).is_some() {
                    CFMutableArray::append_value(
                        Some(fallback_array),
                        desc as *const _ as *const c_void,
                    );
                }
            }
        }
    }
}
