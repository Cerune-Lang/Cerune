/// 変換の意味とは別に、ソース上で選ばれた書式を保持します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionSyntax {
    /// `i64(value)`という簡潔な書式です。
    Compact,
    /// `convert<i64>(value)`という操作名を明示した書式です。
    Explicit,
}

/// ソースコード上のUTF-8バイト範囲を表します。
///
/// `start` は範囲に含み、`end` は含みません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    source_id: SourceId,
    start: usize,
    end: usize,
}

/// ソースコード上の1から始まる行番号と列番号を表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    line: usize,
    column: usize,
}

impl SourceLocation {
    /// UTF-8バイト位置に対応する行番号と列番号を求めます。
    ///
    /// 列番号はUnicodeスカラー値単位で数えます。
    /// 位置がソースの範囲外またはUTF-8文字の途中の場合は`None`を返します。
    pub fn from_offset(source: &str, offset: usize) -> Option<Self> {
        let prefix = source.get(..offset)?;

        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit('\n')
            .next()
            .expect("split always contains at least one element")
            .chars()
            .count()
            + 1;

        Some(Self { line, column })
    }

    /// 1から始まる行番号を返します。
    pub const fn line(self) -> usize {
        self.line
    }

    /// 1から始まる列番号を返します。
    pub const fn column(self) -> usize {
        self.column
    }
}

impl Span {
    /// 指定した開始位置と終了位置から範囲を作成します。
    pub const fn new(start: usize, end: usize) -> Self {
        Self::in_source(SourceId::ANONYMOUS, start, end)
    }

    /// 同じファイル内のUTF-8バイト範囲を作成します。
    pub const fn in_source(source_id: SourceId, start: usize, end: usize) -> Self {
        assert!(start <= end, "span start must not exceed end");

        Self {
            source_id,
            start,
            end,
        }
    }

    pub const fn source_id(self) -> SourceId {
        self.source_id
    }

    /// 字句解析直後に、文字列内の位置へファイルの識別子を付けます。
    pub const fn with_source(self, source_id: SourceId) -> Self {
        Self { source_id, ..self }
    }

    /// 指定した位置に空の範囲を作成します。
    pub const fn empty(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    /// 範囲の開始バイト位置を返します。
    pub const fn start(self) -> usize {
        self.start
    }

    /// 範囲の終了バイト位置を返します。
    pub const fn end(self) -> usize {
        self.end
    }
}

/// 一つのSourceMap内で有効な識別子です。0は従来の匿名ソースを表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(usize);

impl SourceId {
    pub const ANONYMOUS: Self = Self(0);

    pub const fn index(self) -> usize {
        self.0
    }

    /// 匿名ソースの既存の観測形式は変えません。
    pub fn record_field(self) -> String {
        if self == Self::ANONYMOUS {
            String::new()
        } else {
            format!(" file={}", self.0)
        }
    }
}

/// ソースの提供元です。名前やパス文字列から推測しません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    User,
    Standard {
        identifier: &'static str,
        version: u32,
    },
}

/// 読み込んだままの本文と表示名。パス解決やファイルI/Oは行いません。
#[derive(Debug, Clone)]
pub struct SourceFile {
    kind: SourceKind,
    id: SourceId,
    name: String,
    text: String,
}

impl SourceFile {
    pub fn kind(&self) -> SourceKind {
        self.kind
    }
    pub const fn id(&self) -> SourceId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// 登録順に識別子を割り当てます。正規化・連結・重複名による上書きはしません。
/// 別のMapや別のコンパイルで得た識別子を混ぜてはいけません。
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        let id = SourceId(self.files.len() + 1);
        self.files.push(SourceFile {
            kind: SourceKind::User,
            id,
            name: name.into(),
            text: text.into(),
        });
        id
    }

    pub(crate) fn add_standard(
        &mut self,
        identifier: &'static str,
        version: u32,
        text: &str,
    ) -> SourceId {
        let id = self.add(identifier, text);
        self.files[id.index() - 1].kind = SourceKind::Standard {
            identifier,
            version,
        };
        id
    }

    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.0.checked_sub(1)?)
    }

    pub fn files(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter()
    }

    /// 範囲全体が本文内のUTF-8境界にあるときだけ解決します。
    pub fn resolve(&self, span: Span) -> Option<(&SourceFile, SourceLocation)> {
        let file = self.get(span.source_id())?;
        file.text.get(span.start()..span.end())?;
        Some((file, SourceLocation::from_offset(&file.text, span.start())?))
    }

    pub fn slice(&self, span: Span) -> Option<&str> {
        self.get(span.source_id())?
            .text
            .get(span.start()..span.end())
    }
}

#[cfg(test)]
mod tests {
    use super::SourceLocation;

    #[test]
    fn locates_start_of_source() {
        let location = SourceLocation::from_offset("", 0).unwrap();

        assert_eq!(location.line(), 1);
        assert_eq!(location.column(), 1);
    }

    #[test]
    fn locates_position_on_second_line() {
        let location = SourceLocation::from_offset("one\ntwo", 5).unwrap();

        assert_eq!(location.line(), 2);
        assert_eq!(location.column(), 2);
    }

    #[test]
    fn counts_unicode_scalar_values_in_column() {
        let location = SourceLocation::from_offset("a\n日本語", 8).unwrap();

        assert_eq!(location.line(), 2);
        assert_eq!(location.column(), 3);
    }

    #[test]
    fn locates_position_after_trailing_newline() {
        let location = SourceLocation::from_offset("x\n", 2).unwrap();

        assert_eq!(location.line(), 2);
        assert_eq!(location.column(), 1);
    }

    #[test]
    fn handles_crlf_line_endings() {
        let location = SourceLocation::from_offset("a\r\nb", 3).unwrap();

        assert_eq!(location.line(), 2);
        assert_eq!(location.column(), 1);
    }

    #[test]
    fn rejects_offset_inside_utf8_character() {
        assert_eq!(SourceLocation::from_offset("日", 1), None);
    }

    #[test]
    fn rejects_offset_past_end() {
        assert_eq!(SourceLocation::from_offset("x", 2), None);
    }
}
