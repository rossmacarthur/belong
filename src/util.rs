pub trait TomlValueExt {
    fn default() -> Self;
}

impl TomlValueExt for toml::Value {
    fn default() -> Self {
        Self::Table(toml::value::Table::default())
    }
}
