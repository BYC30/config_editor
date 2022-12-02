use thiserror::Error;

#[derive(Error, Debug)]

pub enum AppError {
    #[error("页签[{0}]未找到")]
    SheetNotFound(String),
    #[error("Excel打开错误")]
    ExcelError(#[from] calamine::XlsxError),
    #[error("表格[{0}]主键未找到")]
    TableKeyNotFound(String),
    #[error("字段类型[{0}]不支持")]
    FieldTypeNotSupport (String),
    #[error("编辑器类型[{0}]不支持")]
    EditorTypeNotSupport (String),
    #[error("未知错误[{0}]")]
    Other(#[from] anyhow::Error),
    #[error("配置表页签[{0}]格式错误")]
    ConfigFormatError(String),
    #[error("导入表格key[{0}]未找到")]
    ImportExcelKeyNotFound(String),
    #[error("文件路径[{0}]未包含Content目录")]
    UEFileContentNotFound(String),
    #[error("文件路径[{0}]非uasset")]
    UEFileNotUasset(String),
    #[error("文件路径[{0}]文件名获取失败")]
    UEFileNameNotFound(String),
    #[error("{0}")]
    HintMsg(String),
}
