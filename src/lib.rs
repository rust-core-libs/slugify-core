use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[derive(Debug, Clone)]
pub struct SlugOptions {
    pub separator: char,
    pub max_length: Option<usize>,
    pub lowercase: bool,
    pub remove_stopwords: bool,
    pub ascii_only: bool,
}

impl Default for SlugOptions {
    fn default() -> Self {
        Self {
            separator: '-',
            max_length: None,
            lowercase: true,
            remove_stopwords: false,
            ascii_only: false,
        }
    }
}

const COMMON_STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from",
    "has", "he", "in", "is", "it", "its", "of", "on", "that", "the",
    "to", "was", "will", "with", "the", "this", "but", "they", "have",
];

pub fn slugify(input: &str, options: &SlugOptions) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut result = input
        .nfc()
        .collect::<String>()
        .unicode_words()
        .filter_map(|word| {
            let word = word.trim();
            if word.is_empty() {
                return None;
            }
            
            if options.remove_stopwords && COMMON_STOPWORDS.contains(&word.to_lowercase().as_str()) {
                return None;
            }
            
            let processed = if options.ascii_only {
                word.chars()
                    .filter_map(|c| {
                        if c.is_ascii_alphanumeric() {
                            Some(c)
                        } else if c.is_alphabetic() {
                            transliterate_char(c)
                        } else {
                            None
                        }
                    })
                    .collect::<String>()
            } else {
                word.chars()
                    .filter(|c| c.is_alphanumeric() || c.is_alphabetic())
                    .collect::<String>()
            };
            
            if processed.is_empty() {
                None
            } else {
                Some(processed)
            }
        })
        .collect::<Vec<_>>()
        .join(&options.separator.to_string());

    if options.lowercase {
        result = result.to_lowercase();
    }

    if let Some(max_len) = options.max_length {
        if result.len() > max_len {
            if let Some((idx, _)) = result.char_indices().nth(max_len) {
                result.truncate(idx);
            }
            
            while result.ends_with(options.separator) {
                result.pop();
            }
        }
    }

    result
}

fn transliterate_char(c: char) -> Option<char> {
    match c {
        'à'..='å' | 'À'..='Å' => Some('a'),
        'ç' | 'Ç' => Some('c'),
        'è'..='ë' | 'È'..='Ë' => Some('e'),
        'ì'..='ï' | 'Ì'..='Ï' => Some('i'),
        'ñ' | 'Ñ' => Some('n'),
        'ò'..='ö' | 'Ò'..='Ö' => Some('o'),
        'ù'..='ü' | 'Ù'..='Ü' => Some('u'),
        'ý' | 'ÿ' | 'Ý' => Some('y'),
        'ß' => Some('s'),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn slugify_simple(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    
    let c_str = unsafe { CStr::from_ptr(input) };
    let input_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let options = SlugOptions::default();
    let result = slugify(input_str, &options);
    
    match CString::new(result) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn slugify_with_options(
    input: *const c_char,
    separator: c_char,
    max_length: i32,
    lowercase: bool,
    remove_stopwords: bool,
    ascii_only: bool,
) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    
    let c_str = unsafe { CStr::from_ptr(input) };
    let input_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let options = SlugOptions {
        separator: separator as u8 as char,
        max_length: if max_length > 0 { Some(max_length as usize) } else { None },
        lowercase,
        remove_stopwords,
        ascii_only,
    };
    
    let result = slugify(input_str, &options);
    
    match CString::new(result) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_slugify() {
        let options = SlugOptions::default();
        assert_eq!(slugify("Hello World", &options), "hello-world");
        assert_eq!(slugify("Test 123", &options), "test-123");
    }

    #[test]
    fn test_unicode_handling() {
        let options = SlugOptions::default();
        assert_eq!(slugify("Café münü", &options), "café-münü");
        
        let ascii_options = SlugOptions { ascii_only: true, ..Default::default() };
        assert_eq!(slugify("Café münü", &ascii_options), "cafe-munu");
    }

    #[test]
    fn test_custom_separator() {
        let options = SlugOptions { separator: '_', ..Default::default() };
        assert_eq!(slugify("Hello World", &options), "hello_world");
    }

    #[test]
    fn test_max_length() {
        let options = SlugOptions { max_length: Some(10), ..Default::default() };
        assert_eq!(slugify("This is a very long title", &options), "this-is-a");
    }

    #[test]
    fn test_stopwords() {
        let options = SlugOptions { remove_stopwords: true, ..Default::default() };
        assert_eq!(slugify("The quick brown fox", &options), "quick-brown-fox");
    }

    #[test]
    fn test_empty_input() {
        let options = SlugOptions::default();
        assert_eq!(slugify("", &options), "");
        assert_eq!(slugify("   ", &options), "");
    }

    #[test]
    fn test_special_characters() {
        let options = SlugOptions::default();
        assert_eq!(slugify("Hello, World! @#$%", &options), "hello-world");
    }
}