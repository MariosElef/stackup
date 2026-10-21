use super::choice::choice;

choice!(Service {
    Postgres => "Postgres",
    Redis => "Redis",
    Mongodb => "MongoDB",
    Keycloak => "Keycloak (auth)",
    Mailtrap => "Mailtrap (email sandbox)",
    Https => "Local HTTPS (Caddy + mkcert)",
});
