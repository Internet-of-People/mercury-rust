use serde_json::json;

use super::SchemaVersion;

pub fn get() -> Vec<SchemaVersion> {
    vec![age_over(), email_address(), full_name()]
}

fn age_over() -> SchemaVersion {
    SchemaVersion::new(
        "McL9746fWtE9EXV5",
        "iop",
        "age-over",
        0,
        json![{
            "type": "object",
            "properties": {
                "age": {
                    "type": "number",
                    "min": 0,
                    "max": 255,
                    "required": true,
                    "description": "Eg.: 42. Pick a number between 0 and 255",
                },
            },
        }],
    )
}

fn email_address() -> SchemaVersion {
    SchemaVersion::new(
        "McL9746fWtE9EXVb",
        "iop",
        "email-address",
        0,
        json![{
            "type": "object",
            "properties": {
                "email": {
                    "type": "string",
                    "regex": r#"/^(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\])|(([a-zA-Z\-0-9]+\.)+[a-zA-Z]{2,}))$/"#,
                    "required": true,
                    "description": "Eg.: john.doe@ecorp-usa.com",
                }
            }
        }],
    )
}

fn full_name() -> SchemaVersion {
    SchemaVersion::new_with_order(
        "McL9746fWtE9EXVa",
        "iop",
        "full-name",
        0,
        json![{
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "maxLength": 50,
                    "required": false,
                    "description": "eg.: Mr.",
                },
                "givenName": {
                    "type": "string",
                    "maxLength": 50,
                    "required": true,
                    "description": "eg.: John",
                },
                "middleName": {
                    "type": "string",
                    "maxLength": 50,
                    "required": false,
                    "description": "eg.: Canary",
                },
                "familyName": {
                    "type": "string",
                    "maxLength": 50,
                    "required": true,
                    "description": "eg.: Doe",
                },
            },
        }],
        vec!["title", "givenName", "middleName", "familyName"],
    )
}
