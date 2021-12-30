pub fn fix_unix_filename(filename: &str) -> String {
    if cfg!(unix) && !filename.starts_with("./") {
        ["./", filename].concat().to_string()
    } else {
        filename.to_string()
    }
}

pub fn fix_unix_filename_vec(filename_vec: &mut Vec<String>) {
    if filename_vec.len() == 1 {
        filename_vec[0] = fix_unix_filename(&filename_vec[0]);
    }
}
