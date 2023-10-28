use syn::meta::ParseNestedMeta;

pub trait ReadAttribute {
    const PATH: &'static str;
    fn read_attr(&mut self, meta: ParseNestedMeta) -> syn::Result<()>;

    fn read_attrs(&mut self, attrs: &[syn::Attribute]) -> syn::Result<()> {
        let filtered = attrs.iter().filter(|a| a.path().is_ident(Self::PATH));
        for attr in filtered {
            attr.parse_nested_meta(|m| self.read_attr(m))?;
        }
        Ok(())
    }
}
