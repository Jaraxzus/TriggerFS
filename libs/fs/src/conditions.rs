use infer::MatcherType;
use regex::Regex;
use serde::Deserialize;
use std::{
    fs::Metadata,
    path::{Path, PathBuf},
};
use tracing::{trace, warn};

pub trait ConditionChecker {
    fn check(&self, args: &CheckArgs) -> bool;
}

#[derive(Debug)]
pub struct CheckArgs {
    pub file_metadata: std::fs::Metadata,
    pub file_type: Option<infer::MatcherType>,
    pub file_path: PathBuf,
}

// Condition представляет все возможные условия
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    FileSystemEntity(FileSystemEntity),
    FileSize(FileSizeCondition),
    FileNamePatternCondition(FileNamePatternCondition),
}

impl Condition {
    pub fn check(&self, args: &CheckArgs) -> bool {
        match self {
            Condition::FileSystemEntity(file_system_entity) => file_system_entity.check(args),
            Condition::FileSize(file_size) => file_size.check(args),
            Condition::FileNamePatternCondition(pattern) => pattern.check(&args.file_path),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct FileNamePatternCondition {
    pub pattern: String, // Регулярное выражение для проверки имени файла и расширения
}

impl FileNamePatternCondition {
    pub fn check(&self, file_path: &Path) -> bool {
        let re = match Regex::new(&self.pattern) {
            Ok(re) => re,
            Err(err) => {
                warn!("Invalid regex pattern: {}", err);
                return false;
            }
        };
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        re.is_match(file_name)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FileSystemEntity {
    File(FileType), // Обычный файл с типом
    Directory,      // Папка
    Symlink,        // Символическая ссылка
}

impl ConditionChecker for FileSystemEntity {
    fn check(&self, args: &CheckArgs) -> bool {
        match &self {
            FileSystemEntity::File(file) => file.check(args),
            FileSystemEntity::Directory => args.file_metadata.is_dir(),
            FileSystemEntity::Symlink => args.file_metadata.is_symlink(),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct FileType {
    // TODO: мб сделать вектор типов
    matcher_type: MatcherTypeInernal,
    operator: ComparisonOperator,
}
impl ConditionChecker for FileType {
    fn check(&self, args: &CheckArgs) -> bool {
        args.file_type
            .map_or(false, |file_type| match &self.operator {
                ComparisonOperator::Equal => MatcherType::from(&self.matcher_type) == file_type,
                ComparisonOperator::NotEqual => MatcherType::from(&self.matcher_type) != file_type,
                _ => {
                    warn!("invalid ComparisonOperator for filetype");
                    false
                }
            })
    }
}
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MatcherTypeInernal {
    App,
    Archive,
    Audio,
    Book,
    Doc,
    Font,
    Image,
    Text,
    Video,
    Custom,
    //TODO: мб нужно добавить тип на случай ошибки потому что такое может быть
}

impl From<&MatcherTypeInernal> for MatcherType {
    fn from(item: &MatcherTypeInernal) -> MatcherType {
        match item {
            MatcherTypeInernal::App => MatcherType::App,
            MatcherTypeInernal::Archive => MatcherType::Archive,
            MatcherTypeInernal::Audio => MatcherType::Audio,
            MatcherTypeInernal::Book => MatcherType::Book,
            MatcherTypeInernal::Doc => MatcherType::Doc,
            MatcherTypeInernal::Font => MatcherType::Font,
            MatcherTypeInernal::Image => MatcherType::Image,
            MatcherTypeInernal::Text => MatcherType::Text,
            MatcherTypeInernal::Video => MatcherType::Video,
            MatcherTypeInernal::Custom => MatcherType::Custom,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
}

impl SizeUnit {
    fn to_bytes(self, size: u64) -> u64 {
        match self {
            SizeUnit::Bytes => size,
            SizeUnit::Kilobytes => size * 1024,
            SizeUnit::Megabytes => size * 1024 * 1024,
            SizeUnit::Gigabytes => size * 1024 * 1024 * 1024,
        }
    }
}
/// FileSizeCondition условия по размеру файла, нужно быть аккуратными с тригерами перед этим
/// условием, так как на момент создания файл может быть не до конца записан, и сравнение будет не
/// корректным, лучше всего использовать условие
/// ```json
/// {
///  "access": {
///   "kind": {
///     "close": "any"
///   }
///  }
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct FileSizeCondition {
    operator: ComparisonOperator,
    size: u64,
    unit: SizeUnit,
}
impl ConditionChecker for FileSizeCondition {
    fn check(&self, args: &CheckArgs) -> bool {
        let res = self.is_satisfied(&args.file_metadata);
        trace!("res: {}", res);
        res
    }
}
impl FileSizeCondition {
    fn is_satisfied(&self, metadata: &Metadata) -> bool {
        let file_size = metadata.len();
        let size_in_bytes = self.unit.to_bytes(self.size);
        trace!("file_size: {}, size_in_bytes: {}", file_size, size_in_bytes);

        match self.operator {
            ComparisonOperator::GreaterThan => file_size > size_in_bytes,
            ComparisonOperator::GreaterThanOrEqual => file_size >= size_in_bytes,
            ComparisonOperator::LessThan => file_size < size_in_bytes,
            ComparisonOperator::LessThanOrEqual => file_size <= size_in_bytes,
            ComparisonOperator::Equal => file_size == size_in_bytes,
            ComparisonOperator::NotEqual => file_size != size_in_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    // TODO:
    // In(Vec<T>),
    // NotIn(Vec<T>),
}
