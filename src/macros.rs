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

macro_rules! send_set_channel{
    {$writer:expr, $channel: expr} => {
        send!($writer, json!({
            "cmd": "SELECT_VOICE_CHANNEL",
            "args": { 
                "channel_id": $channel,
                "force": true
            },
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

// Cairo helpers
 
macro_rules! draw_overlay{
    {$window: expr, $ctx: expr, $avatar_list:expr, $avatar_list_raw:expr, $state: expr} => {
        let reg = Region::create();
        reg.union_rectangle(& RectangleInt{
            x: 0,
            y: 0,
            width:1,
            height:1
        }).expect("Failed to add rectangle"); 
        // Config / Static
        let edge = 6.0;
        let line_height = 32.0;

        $ctx.set_antialias(Antialias::Good);
        $ctx.set_operator(Operator::Source);
        $ctx.set_source_rgba(1.0, 0.0, 0.0, 0.0);
        $ctx.paint().expect("Unable to paint window");

        $ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
        $ctx.set_font_size(16.0);
        let state = $state.lock().unwrap().clone();
        let mut y = 50.0;
        $ctx.set_operator(Operator::Over);

        if state.users.len() > 0 {
            for (key, user) in state.users {
                match state.voice_states.get(&key) {
                    Some(voice_state) => {
                        let mut name = user.username.clone();
                        match &voice_state.nick {
                            Some(nick) => {
                                name = nick.clone();
                            }
                            None => {}
                        }
                        if voice_state.talking {
                            $ctx.set_source_rgba(0.0, 0.4, 0.0, 0.6);
                        } else {
                            $ctx.set_source_rgba(0.0, 0.0, 0.0, 0.4);
                        }
                        let ext = $ctx.text_extents(&name).unwrap();
                        // Draw border around text
                        $ctx.rectangle(
                            line_height,
                            y + (line_height / 2.0) - (ext.height / 2.0) - edge,
                            ext.width + edge * 2.0,
                            ext.height + edge * 2.0,
                        );
                        $ctx.fill().expect("Unable to fill");
                        $ctx.move_to(
                            line_height + edge,
                            y + (line_height / 2.0) + (ext.height / 2.0),
                        );
                        // Draw border into XShape
                        reg.union_rectangle(& RectangleInt{
                            x:line_height as i32 ,
                            y:(y + (line_height / 2.0) - (ext.height / 2.0) - edge) as i32,
                            width: (ext.width + edge * 2.0) as i32,
                            height: (ext.height + edge * 2.0) as i32
                        }).expect("Unable to add rectangle to XShape");
                        reg.union_rectangle(& RectangleInt{
                            x:0,
                            y:y as i32 ,
                            width:line_height as i32,
                            height:line_height as i32
                        }).expect("Unable to add rectangle to XShape");

                        if voice_state.talking {
                            $ctx.set_source_rgba(0.0, 1.0, 0.0, 1.0);
                        } else {
                            $ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                        }
                        $ctx.show_text(&name).expect("unable to draw text");
                        if voice_state.deaf || voice_state.self_deaf {
                            draw_deaf($ctx, 0.0, y, line_height);
                        } else if voice_state.mute || voice_state.self_mute {
                            draw_mute($ctx, 0.0, y, line_height);
                        }
                        let mut avatar_list = $avatar_list.lock().unwrap();
                        let avatar_list_raw = $avatar_list_raw.lock().unwrap();
                        match avatar_list.get(&user.id){
                            Some(img)=>{
                                match img{
                                    Some(img) =>{
                                        $ctx.save().expect("Unable to save cairo state");
                                        $ctx.translate(0.0, y);
                                        $ctx.scale(line_height, line_height);
                                        $ctx.scale(1.0 / img.width() as f64, 1.0 / img.height() as f64);
                                        $ctx.set_source_surface(img,0.0,0.0).unwrap();
                                        $ctx.rectangle(0.0,0.0,img.width() as f64, img.height() as f64);
                                        $ctx.fill().unwrap();
                                        $ctx.restore().expect("Unable to restore cairo state");
                                    }
                                    None => {
                                        // Requested but no image (yet?) Don't draw anything more
                                    }
                                }
                            }
                            None=>{
                                // Not requested yet. Don't draw anything
                                match avatar_list_raw.get(&user.id){
                                    Some(maybe_raw) => {
                                        match maybe_raw{
                                            Some(raw) => {
                                                let surface = ImageSurface::create_from_png(&mut Cursor::new(raw)).expect("Error processing user avatar");
                                                avatar_list.insert(user.id.clone(), Some(surface));
                                            }
                                            None => {
                                            }
                                        }
                                    }
                                    None => {
                                    }
                                }
                            }
                        }
                    }
                    None => {}
                }
                y += line_height;
            }
        }
        $window.shape_combine_region(Some(&reg));
    }
}