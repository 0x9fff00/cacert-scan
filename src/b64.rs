use base64::{
    engine::general_purpose::{
        STANDARD as BASE64_STANDARD, STANDARD_NO_PAD as BASE64_STANDARD_NO_PAD,
    },
    Engine,
};
use memchr::memmem;

fn clean_b64(body: &[u8]) -> Vec<u8> {
    // tolerate PEM embedded in C/JS/Go string literals: "...\n" + "..."
    let mut out = Vec::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        let c = body[i];
        if c == b'\\' && i + 1 < body.len() {
            i += 2;
            continue;
        }
        if c == b'"' || c == b'\'' {
            // skip a string-literal break such as `" +\n  "` or `"\n"`
            let mut j = i + 1;
            while j < body.len()
                && matches!(
                    body[j],
                    b' ' | b'\t' | b'\r' | b'\n' | b'+' | b',' | b'&' | b'.' | b'_'
                )
            {
                j += 1;
            }
            if j < body.len() && (body[j] == b'"' || body[j] == b'\'') {
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }
        if c.is_ascii_alphanumeric() || c == b'+' || c == b'/' || c == b'=' {
            out.push(c);
        }
        i += 1;
    }
    out
}

pub fn b64_decode(s: &[u8]) -> Option<Vec<u8>> {
    let e = &BASE64_STANDARD;
    e.decode(s).ok().or_else(|| {
        let t: Vec<u8> = s.iter().copied().filter(|&c| c != b'=').collect();
        BASE64_STANDARD_NO_PAD.decode(&t).ok()
    })
}

/// Returns (start, decoded DER if the body decoded) for every certificate PEM block.
pub fn pem_blocks(data: &[u8]) -> Vec<(usize, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let begin = memmem::Finder::new(b"-----BEGIN ");
    let end = memmem::Finder::new(b"-----END ");
    let mut pos = 0;
    while let Some(b) = begin.find(&data[pos..]) {
        let start = pos + b;
        let label_end = if let Some(e) =
            memmem::find(&data[start + 11..(start + 80).min(data.len())], b"-----")
        {
            start + 11 + e
        } else {
            pos = start + 11;
            continue;
        };
        let label = &data[start + 11..label_end];
        let body_start = label_end + 5;
        let Some(e) = end.find(&data[body_start..(body_start + 64 * 1024).min(data.len())]) else {
            pos = body_start;
            continue;
        };
        pos = body_start + e + 9;
        if !label.ends_with(b"CERTIFICATE") {
            continue;
        }
        let der = b64_decode(&clean_b64(&data[body_start..body_start + e]));
        out.push((start, der));
    }
    out
}
