use actix_web::{get, HttpResponse};

static OPENAPI_YAML: &str = include_str!("../../Punchcraft-openapi.yaml");

#[get("/api-docs/openapi.yaml")]
pub async fn serve_spec() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/yaml")
        .body(OPENAPI_YAML)
}

#[get("/api-docs")]
pub async fn serve_ui() -> HttpResponse {
    let html = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>PunchCraft API Docs</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: "/api-docs/openapi.yaml",
      dom_id: "#swagger-ui",
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: "BaseLayout",
      deepLinking: true,
    });
  </script>
</body>
</html>"##;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(serve_spec).service(serve_ui);
}
