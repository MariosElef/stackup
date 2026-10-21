use super::choice::choice;

choice!(Frontend { Nuxt => "Nuxt", Vue => "Vue (Vite)", None => "none" });
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
