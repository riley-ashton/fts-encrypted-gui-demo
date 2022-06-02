#![allow(non_snake_case)]

mod enron_emails;

use dioxus::{
    core::{
        exports::futures_channel::{self, mpsc::unbounded},
        to_owned,
    },
    prelude::*,
};
use futures::StreamExt;
use futures_channel::mpsc::UnboundedReceiver;
use secrecy::Secret;
use std::{
    cell::Cell,
    cmp::min,
    str::from_utf8,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::oneshot;

use enron_emails::enron_emails;
use fts_encrypted::symmetric_key::SymmetricKey;
use fts_encrypted::{doc_id::DocId, fts::Fts};

/// A dummy key used for this example.
/// Obviously a hardcoded key would not be used in a real app.
/// Using a user provided passphrase with a key derivation function,
/// like Argon2 or a key from elsewhere would be a good choice.
const DUMMY_KEY: [u8; 16] = [
    188, 205, 168, 149, 43, 16, 205, 172, 224, 128, 96, 213, 214, 18, 45, 198,
];

fn main() {
    let db = sled::open("db").unwrap();
    let email_db = Arc::new(db.open_tree("emails").unwrap());
    let fts = Arc::new(Fts::new_default(&db, DUMMY_KEY));
    let key = Secret::new(SymmetricKey(DUMMY_KEY.into()));
    let key_ = Secret::new(SymmetricKey(DUMMY_KEY.into()));

    let (sender, receiver) = unbounded();

    let email_db_writer = email_db.clone();
    let fts_writer = fts.clone();

    // Indexing handler
    std::thread::spawn(move || {
        // already done
        if email_db_writer.len() > 9000 {
            // Change state
            return;
        }

        let emails = enron_emails();
        let total = emails.len();
        // TODO set emails.len() as denominator in gui progress bar

        for (count, (id, content)) in emails.into_iter().enumerate() {
            let encrypted_id = id.clone().encrypt(&key);

            email_db_writer
                .insert(encrypted_id, content.as_bytes())
                .unwrap();

            fts_writer.add_document(id, content).unwrap();
            let _ = sender.unbounded_send((count, total));
        }
    });

    let (search, mut responder) = unbounded::<(SearchPhrase, oneshot::Sender<Vec<EmailContent>>)>();

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Search handler
    rt.spawn(async move {
        while let Some((term, callback)) = responder.next().await {
            let ids = fts.search(term).unwrap();
            let content = ids
                .into_iter()
                .map(|id| get_email(id, &key_, &email_db))
                .collect();
            callback.send(content).unwrap();
        }
    });

    let props = AppProps {
        receiver: Cell::new(Some(receiver)),
        search: Arc::new(Mutex::new(search)),
    };

    dioxus::desktop::launch_with_props(app, props, |c| {
        c.with_window(|w| w.with_title("fts-encrypted demo"))
    });
}

type EmailContent = String;
type SearchPhrase = String;
type SearchResponse = oneshot::Sender<Vec<EmailContent>>;

struct AppProps {
    receiver: Cell<Option<UnboundedReceiver<(usize, usize)>>>,
    search: Arc<Mutex<UnboundedSender<(SearchPhrase, SearchResponse)>>>,
}

/// Get an email from the encrypted email db
fn get_email(id: DocId, key: &Secret<SymmetricKey>, db: &sled::Tree) -> EmailContent {
    let encrypted_id = id.encrypt(key);
    let bytes = db.get(encrypted_id).unwrap().unwrap();
    // TODO decrypt
    from_utf8(&bytes[..])
        .unwrap_or("ENCODING ERROR")
        .to_string()
}

fn app(cx: Scope<AppProps>) -> Element {
    let page = use_state(&cx, || AppState::Indexing);
    let progress = use_state(&cx, || 0.0);
    let total = use_state(&cx, || 0);

    let _ = use_coroutine(&cx, |_: UnboundedReceiver<bool>| {
        let receiver = cx.props.receiver.take();
        to_owned![progress, page, total];
        async move {
            if let Some(mut receiver) = receiver {
                while let Some((done, total_)) = receiver.next().await {
                    progress.set(done as f32 * 100.0 / total_ as f32);
                    if total == 0 {
                        total.set(total_);
                    }
                }
                page.set(AppState::Ready)
            }
        }
    });

    match page.current().as_ref() {
        AppState::Indexing => rsx!(cx, div {
            display: "flex",
            flex_direction: "column",
            align_items: "center",

            div {
                display: "flex",
                align_items: "center",
                img { src: "./logo.png", padding_right: "12px", width: "50px" }
                h1 { "fts-encrypted demo" }
            }

            p { "Processing {total} emails" }
            div {
                background_color: "grey",
                width: "300px",
                height: "28px",
                class: "silver",
                border_radius: "16px",
                div {
                    class: "bar",
                    background_color: "lightskyblue",
                    width: "{progress}%",
                    height: "28px",
                    border_radius: "16px",
                }
            }
            Counter {}
        }),
        AppState::Ready => {
            let search_input = use_state(&cx, || "".to_string());

            let search_result = use_future(&cx, search_input, |search_input| {
                let search = cx.props.search.clone();
                async move {
                    if search_input.len() > 3 {
                        let (tx, rx) = oneshot::channel();

                        {
                            let mut guard = search.lock().unwrap();
                            guard.start_send((search_input.get().clone(), tx)).unwrap();
                        }

                        rx.await.unwrap()
                    } else {
                        vec![]
                    }
                }
            });

            let results_ui = match search_result.value() {
                Some(results) => rsx!(cx, div {
                    Results { results: results }
                }),
                None => rsx!(cx, div {}),
            };

            rsx!(cx, div {
                h3 { "Full text search of encrypted emails" }
                input {
                    "type": "test",
                    value: "{search_input}",
                    placeholder: "Search encrypted emails",
                    oninput: move |evt| search_input.set(evt.value.clone()),
                }
                results_ui
            })
        }
    }
}

const SUMMARY_LEN: usize = 2500;

#[inline_props]
fn Results<'a>(cx: Scope, results: &'a Vec<EmailContent>) -> Element {
    let length = results.len().to_string();

    let list = results.iter().map(|content| {
        let length = min(SUMMARY_LEN, content.len());
        let summary = &content[0..length];

        // Add a suffix if the email is cut off
        let suffix = if length == SUMMARY_LEN {
            "...(over 2500 characters)..."
        } else {
            ""
        };

        rsx!( li { "{summary} {suffix}" })
    });

    rsx!(cx, div {
        "Returned {length} emails"
        ol {
            list
        }
    })
}

fn Counter(cx: Scope) -> Element {
    let elapsed = use_state(&cx, || 0);

    cx.spawn({
        to_owned![elapsed];

        async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            elapsed += 1;
        }
    });

    rsx!(cx,
            div {
                p { "Time elapsed: {elapsed} seconds" }
            }
    )
}

#[derive(PartialEq)]
enum AppState {
    Indexing,
    Ready,
}
