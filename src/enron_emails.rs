use std::collections::HashMap;

use fts_encrypted::doc_id::DocId;

pub(crate) fn enron_emails() -> HashMap<DocId, String> {
    let file = std::fs::read_to_string("./test_emails/cleaned_emails.csv").unwrap();
    let mut lines = file.lines();
    let _ = lines.next();

    let mut emails = HashMap::new();

    for line in lines {
        let fields = line.split(',');
        let mut content = "".to_string();

        for (i, field) in fields.enumerate() {
            if i != 5 {
                String::push_str(&mut content, field);
            }
        }

        let id: DocId = uuid::Uuid::new_v4().into();
        emails.insert(id, content);
    }

    emails
}
