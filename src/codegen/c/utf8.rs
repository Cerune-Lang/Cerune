pub(super) const SUPPORT: &str = r#"
static int cerune_utf8(const unsigned char *data, size_t length) {
    size_t i = 0;
    while (i < length) {
        unsigned char a = data[i++];
        if (a <= 0x7f) continue;
        size_t rest;
        unsigned char low = 0x80, high = 0xbf;
        if (a >= 0xc2 && a <= 0xdf) rest = 1;
        else if (a >= 0xe0 && a <= 0xef) {
            rest = 2;
            if (a == 0xe0) low = 0xa0;
            if (a == 0xed) high = 0x9f;
        } else if (a >= 0xf0 && a <= 0xf4) {
            rest = 3;
            if (a == 0xf0) low = 0x90;
            if (a == 0xf4) high = 0x8f;
        } else return 0;
        if (rest > length - i || data[i] < low || data[i] > high) return 0;
        ++i;
        while (--rest) {
            if (data[i] < 0x80 || data[i] > 0xbf) return 0;
            ++i;
        }
    }
    return 1;
}
"#;
