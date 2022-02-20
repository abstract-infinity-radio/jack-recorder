pub fn is_unsafe_char(x: char) -> bool {
    if (x >= 'a' && x <= 'z') || (x >= 'A' && x <= 'Z') || (x >= '0' && x <= '9') {
        return false;
    }
    return true;
}
