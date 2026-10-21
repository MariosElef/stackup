use super::choice::choice;

choice!(Frontend { Nuxt => "Nuxt", Vue => "Vue (Vite)", Topcoat => "Topcoat (Rust)", None => "none" });
choice!(FeLib {
    Pinia => "Pinia (state)",
    Colada => "Pinia Colada (queries)",
    I18n => "i18n (translations)",
    Zod => "Zod (validation)",
    Openapi => "OpenAPI client (hey-api)",
    Lucide => "Lucide icons",
    Tailwind => "Tailwind CSS 4",
    Vueuse => "VueUse",
});

impl FeLib {
    /// Most libraries are npm packages for Nuxt or Vue. Topcoat is Rust: it builds Tailwind and
    /// Lucide (through Iconify) itself, and i18n is a crate.
    pub fn works_with(self, frontend: Frontend) -> bool {
        match frontend {
            Frontend::Nuxt | Frontend::Vue => true,
            Frontend::Topcoat => matches!(self, FeLib::Tailwind | FeLib::Lucide | FeLib::I18n),
            Frontend::None => false,
        }
    }
}
