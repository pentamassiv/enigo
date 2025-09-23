use enigo::agent::Token;
use serde::{Deserialize, Serialize};
use tungstenite::Message;

#[derive(Debug, Clone, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserEvent {
    Syn(u32, Token), // ID, Token
    Ack(u32, Token), // ID, Token
    Open,
    Close,
}

/// Manual impl so that BrowserEvent::Syn and BrowserEvent::Ack are equal if the
/// ID and token are equal
impl PartialEq for BrowserEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Syn(l0, l1), Self::Ack(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Syn(l0, l1), Self::Syn(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Ack(l0, l1), Self::Ack(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Ack(l0, l1), Self::Syn(r0, r1)) => l0 == r0 && l1 == r1,
            (BrowserEvent::Open, BrowserEvent::Open) => true,
            (BrowserEvent::Close, BrowserEvent::Close) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserEventError {
    UnknownMessageType,
    ParseError,
}

impl TryFrom<Message> for BrowserEvent {
    type Error = BrowserEventError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::Close(_) => {
                println!("Message::Close received");
                Ok(BrowserEvent::Close)
            }
            Message::Text(msg) => {
                println!("Browser received input");
                println!("msg: {msg:?}");

                // Attempt to deserialize the text message into a BrowserEvent
                if let Ok(event) = ron::from_str::<BrowserEvent>(&msg) {
                    Ok(event)
                } else {
                    println!("Parse error! Message: {msg}");
                    Err(BrowserEventError::ParseError)
                }
            }
            Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                println!("Other Message received");
                Err(BrowserEventError::UnknownMessageType)
            }
        }
    }
}

/*

#[test]
fn deserialize_browser_events() {
    /*
    Syn(u32, Token),
    Ack(u32, Token),
     */
    let messages = vec![
        (
            Message::Text(Utf8Bytes::from("ReadyForText")),
            BrowserEvent::Syn(1, Token::Text("abcd".to_string())),
        ),
        (
            Message::Text(Utf8Bytes::from("Text(\"Testing\")")),
            BrowserEvent::Text("Testing".to_string()),
        ),
        (
            Message::Text(Utf8Bytes::from("Text(\"Hi how are you?❤️ äüß$3\")")),
            BrowserEvent::Text("Hi how are you?❤️ äüß$3".to_string()),
        ),
        (
            Message::Text(Utf8Bytes::from("KeyDown(\"F11\", \"\")")),
            BrowserEvent::KeyDown("F11".to_string(), "".to_string()),
        ),
        (
            Message::Text(Utf8Bytes::from("KeyUp(\"F11\", \"\")")),
            BrowserEvent::KeyUp("F11".to_string(), "".to_string()),
        ),
        (
            Message::Text(Utf8Bytes::from(
                "KeyDown(\"F1\", \"key: F1, which: 112, charCode: 0, shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, repeat: false, isComposing: false, location: 0, bubbles: true, cancelable: true, defaultPrevented: false, composed: true\")",
            )),
            BrowserEvent::KeyDown("F1".to_string(),  "key: F1, which: 112, charCode: 0, shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, repeat: false, isComposing: false, location: 0, bubbles: true, cancelable: true, defaultPrevented: false, composed: true".to_string()),
        ),
        (
            Message::Text(Utf8Bytes::from("MouseDown(0)")),
            BrowserEvent::MouseDown(0),
        ),
        (
            Message::Text(Utf8Bytes::from("MouseUp(0)")),
            BrowserEvent::MouseUp(0),
        ),
        (
            Message::Text(Utf8Bytes::from("MouseMove((-1806, -487), (200, 200))")),
            BrowserEvent::MouseMove((-1806, -487), (200, 200)),
        ),
        (
            Message::Text(Utf8Bytes::from("MouseScroll(3, -2)")),
            BrowserEvent::MouseScroll(3, -2),
        ),
    ];

    for (msg, event) in messages {
        let serialized = ron::to_string(&event).unwrap();
        println!("serialized = {serialized}");

        assert_eq!(BrowserEvent::try_from(msg).unwrap(), event);
    }
}

*/
