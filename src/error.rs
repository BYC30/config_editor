use thiserror::Error;

#[derive(Error, Debug)]

pub enum AppError {
    #[error("页签[{0}]未找到")]
    SheetNotFound(String),
    #[error("Excel打开错误")]
    ExcelError(#[from] calamine::XlsxError),
    #[error("类型[{0}]不支持")]
    FieldTypeNotSupport (String),
    #[error("未知错误[{0}]")]
    Other(#[from] anyhow::Error),
    #[error("加载Excel没有页签")]
    LoadExcelSheetEmpty,
}
