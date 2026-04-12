pub fn text_from_response(mut res: raijin_open_ai::Response) -> Option<String> {
    let choice = res.choices.pop()?;
    let output_text = match choice.message {
        raijin_open_ai::RequestMessage::Assistant {
            content: Some(raijin_open_ai::MessageContent::Plain(content)),
            ..
        } => content,
        raijin_open_ai::RequestMessage::Assistant {
            content: Some(raijin_open_ai::MessageContent::Multipart(mut content)),
            ..
        } => {
            if content.is_empty() {
                log::error!("No output from Baseten completion response");
                return None;
            }

            match content.remove(0) {
                raijin_open_ai::MessagePart::Text { text } => text,
                raijin_open_ai::MessagePart::Image { .. } => {
                    log::error!("Expected text, got an image");
                    return None;
                }
            }
        }
        _ => {
            log::error!("Invalid response message: {:?}", choice.message);
            return None;
        }
    };
    Some(output_text)
}
