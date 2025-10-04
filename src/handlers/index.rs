use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};

pub async fn index() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>lila</title>
    <style>
        @font-face {
            font-family: 'Iosevka Term';
            src: url('https://github.com/aprlpet/aprlpet/raw/refs/heads/mommy/Iosevka.ttf') format('truetype');
            font-weight: 400;
            font-display: swap;
        }

        :root {
            --color-bg: #100F0F;
            --color-bg-2: #1C1B1A;
            --color-ui: #282726;
            --color-ui-2: #343331;
            --color-ui-3: #403E3C;
            --color-tx-3: #575653;
            --color-tx-2: #878580;
            --color-tx: #CECDC3;
            --color-re: #D14D41;
            --color-or: #DA702C;
            --color-ye: #D0A215;
            --color-gr: #879A39;
            --color-cy: #3AA99F;
            --color-bl: #4385BE;
            --color-pu: #8B7EC8;
            --color-ma: #CE5D97;
        }

        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        html, body {
            height: 100%;
        }

        body {
            font-family: 'Iosevka Term', monospace;
            font-weight: 400;
            background: var(--color-bg);
            color: var(--color-tx);
            line-height: 1.6;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 2rem;
        }

        .container {
            max-width: 700px;
        }

        h1 {
            font-size: 2rem;
            color: var(--color-tx);
            font-weight: 400;
            margin-bottom: 0.5rem;
        }

        p {
            margin-bottom: 1.25rem;
            line-height: 1.7;
        }

        .intro {
            margin-bottom: 0;
        }

        a {
            color: var(--color-cy);
            text-decoration: none;
            transition: color 0.2s ease;
        }

        a:hover {
            color: var(--color-cy);
            opacity: 0.7;
        }

        .footer {
            color: var(--color-tx-2);
        }

        @media (max-width: 768px) {
            body {
                padding: 1.5rem 1rem;
            }
            
            h1 {
                font-size: 1.5rem;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>lila</h1>
        
        <div class="intro">
            <p>Object storage powered by Rust. Meant to be lightweight, simple & easy to use</p>
        </div>

        <div class="footer">
            <p>Built by <a href="https://github.com/aprlpet">april</a> · <a href="https://github.com/aprlpet/lila">GitHub</a></p>
        </div>
    </div>
</body>
</html>
"#,
    )
}

pub async fn favicon() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "")
}

pub async fn github_redirect() -> Redirect {
    Redirect::permanent("https://github.com/aprlpet/lila")
}
