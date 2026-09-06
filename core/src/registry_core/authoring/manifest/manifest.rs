// Stable registration-face metadata model.
// 稳定的注册面元数据模型。

use std::collections::BTreeMap;

/// Parsed registration-face metadata shared by authoring and Studio.
/// authoring 与 Studio 共用的注册面元数据模型。
#[derive(Clone, Debug)]
pub(crate) struct FaceManifest {
    pub(crate) values: BTreeMap<String, String>,
}

impl FaceManifest {
    /// Read one normalized field without exposing the backing map.
    /// 读取规范化字段，但不暴露底层 map。
    #[allow(dead_code)]
    pub(crate) fn get(&self, field: &str) -> Option<&str> {
        self.values.get(field).map(String::as_str)
    }

    /// Return a stable, sorted view for Studio and diagnostics.
    /// 为 Studio 和诊断提供稳定排序的只读视图。
    #[allow(dead_code)]
    pub(crate) fn fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}
