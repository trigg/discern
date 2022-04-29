macro_rules! send {
    ($writer: expr, $value: expr) => {
        $writer
            .lock()
            .await
            .send(Message::Text($value.to_string() + "\n"))
            .await
            .unwrap();
    };
}

macro_rules! send_auth{
    ($writer: expr, $auth_code: expr) => {
        send!($writer, json!({
            "cmd": "AUTHORIZE",
            "args": {
                "client_id": $auth_code,
                "scopes": ["rpc", "messages.read"],
                "prompt": "none",
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_auth2{
    ($writer: expr, $token: expr) => {
        send!($writer, json!({
            "cmd": "AUTHENTICATE",
            "args": {
                "access_token": $token,
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_req_all_guilds{
    {$writer: expr} => {
        send!($writer, json!({
            "cmd": "GET_GUILDS",
            "args": {
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_req_selected_voice{
    {$writer: expr} => {
        send!($writer, json!({
            "cmd": "GET_SELECTED_VOICE_CHANNEL",
            "args": {
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_sub{
    {$writer: expr, $event: expr, $args: expr, $nonce: expr} =>{
        send!($writer, json!({
            "cmd": "SUBSCRIBE",
            "args": $args,
            "evt": $event,
            "nonce": $nonce
        }));
    }
}

macro_rules! send_sub_server{
    {$writer: expr} => {
        send_sub!($writer.clone(), "VOICE_CHANNEL_SELECT", json!({}), "VOICE_CHANNEL_SELECT");
        send_sub!($writer.clone(), "VOICE_CONNECTION_STATUS", json!({}), "VOICE_CONNECTION_STATUS");
    }
}

macro_rules! send_sub_channel{
    {$writer: expr, $event: expr, $channel: expr} => {
        send_sub!($writer, $event, json!({"channel_id":$channel}), $channel);
    }
}

macro_rules! send_sub_voice_channel{
    {$writer: expr, $channel: expr} => {
        send_sub_channel!($writer.clone(), "VOICE_STATE_CREATE", $channel);
        send_sub_channel!($writer.clone(), "VOICE_STATE_UPDATE", $channel);
        send_sub_channel!($writer.clone(), "VOICE_STATE_DELETE", $channel);
        send_sub_channel!($writer.clone(), "SPEAKING_START", $channel);
        send_sub_channel!($writer.clone(), "SPEAKING_STOP", $channel);
    }
}

macro_rules! send_req_devices{
    {$writer: expr} =>{
        send!($writer, json!({
            "cmd": "GET_VOICE_SETTINGS",
            "args": {},
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_set_devices{
    {$writer: expr, $dev: expr, $value: expr, $nonce: expr} =>{
        send!($writer, json!({
            "cmd": "SET_VOICE_SETTINGS",
            "args": {$dev:$value},
            "nonce": $nonce
        }));
    }
}
