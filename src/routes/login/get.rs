use actix_web::{HttpResponse, http::header::ContentType};
use actix_web::cookie::Cookie;
use actix_web_flash_messages::{IncomingFlashMessages};
use std::fmt::Write;

pub async fn login_form(
    flash_messages: IncomingFlashMessages
) -> HttpResponse {
    let mut error_html = String::new();

    let flash_messages = flash_messages.iter();
    for flash_message in flash_messages {
        writeln!(error_html, "<p><i>{}</i></p>", flash_message.content()).unwrap();
    }

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body_html_content(error_html));

    response.add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();

    response
}

fn body_html_content(error_html: String) -> String {
    return format!(
        r#"<!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Login</title>
                </head>
                <body>
                    {error_html}
                    <form action="/login" method="post">
                        <label>Username
                            <input
                                type="text
                                placeholder="Enter Username"
                                name="username"
                            >
                        </label>
                        <label>Password
                            <input
                                type="password"
                                placeholder="Enter Password"
                                name="password"
                            >
                        </label>
                        <button type="submit">Login</button>
                    </form>
                </body>
            </html>"#
    )
}