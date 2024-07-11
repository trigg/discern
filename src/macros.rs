// Send raw value over websocket
#[macro_export]
macro_rules! send_socket {
    ($writer: expr, $value: expr) => {
        for packet in $value.iter() {
            $writer
                .lock()
                .await
                .send(Message::Text(packet.to_string() + "\n"))
                .await
                .unwrap();
        }
    };
}

// Send raw value locally to socket thread
#[macro_export]
macro_rules! send_mpsc {
    ($writer: expr, $value: expr) => {
        for packet in $value.iter() {
            $writer
                .lock()
                .await
                .try_send(packet.to_string())
                .expect("Unable to send packet");
        }
    };
}

// First packet to send. Explains what scopes the app will need
#[macro_export]
macro_rules! packet_auth{
    ($auth_code: expr) => {
        [json!({
            "cmd": "AUTHORIZE",
            "args": {
                "client_id": $auth_code,
                "scopes": ["rpc", "messages.read", "rpc.notifications.read"],
                "prompt": "none",
            },
            "nonce": "deadbeef"
        })]
    }
}

// Second, with an access token to authenticate the app
#[macro_export]
macro_rules! packet_auth2{
    ($token: expr) => {
        [json!({
            "cmd": "AUTHENTICATE",
            "args": {
                "access_token": $token,
            },
            "nonce": "deadbeef"
        })]
    }
}

// Request a list of all guilds the user is in
#[macro_export]
macro_rules! packet_req_all_guilds{
    {} => {
        [json!({
            "cmd": "GET_GUILDS",
            "args": {
            },
            "nonce": "deadbeef"
        })]
    }
}

// Request information on the channel the user is currently in
#[macro_export]
macro_rules! packet_req_selected_voice{
    {} => {
        [json!({
            "cmd": "GET_SELECTED_VOICE_CHANNEL",
            "args": {
            },
            "nonce": "deadbeef"
        })]
    }
}

// Subscribe to event callbacks
#[macro_export]
macro_rules! packet_sub{
    {$event: expr, $args: expr, $nonce: expr} =>{
        json!({
            "cmd": "SUBSCRIBE",
            "args": $args,
            "evt": $event,
            "nonce": $nonce
        })
    }
}

// Subscribe to server events
#[macro_export]
macro_rules! packet_sub_server{
    {} => {
        [packet_sub!("VOICE_CHANNEL_SELECT", json!({}), "VOICE_CHANNEL_SELECT"),
        packet_sub!("VOICE_CONNECTION_STATUS", json!({}), "VOICE_CONNECTION_STATUS")]
    }
}

// Subscribe to a channel event
#[macro_export]
macro_rules! packet_sub_channel{
    {$event: expr, $channel: expr} => {
        packet_sub!($event, json!({"channel_id":$channel}), $channel)
    }
}

// Subscribe to voice channel events
#[macro_export]
macro_rules! packet_sub_voice_channel{
    {$channel: expr} => {
        [packet_sub_channel!("VOICE_STATE_CREATE", $channel),
        packet_sub_channel!("VOICE_STATE_UPDATE", $channel),
        packet_sub_channel!("VOICE_STATE_DELETE", $channel),
        packet_sub_channel!("SPEAKING_START", $channel),
        packet_sub_channel!("SPEAKING_STOP", $channel)]
    }
}

// Request information about audio devices
#[macro_export]
macro_rules! packet_req_devices{
    {} =>{
        [json!({
            "cmd": "GET_VOICE_SETTINGS",
            "args": {},
            "nonce": "deadbeef"
        })]
    }
}

// Request we move the user into the channel with the given ID
#[macro_export]
macro_rules! packet_set_channel{
    {$channel: expr} => {
        [json!({
            "cmd": "SELECT_VOICE_CHANNEL",
            "args": {
                "channel_id": $channel,
                "force": true
            },
            "nonce": "deadbeef"
        })]
    }
}

// Request we change the users device setting (mute, deaf etc)
#[macro_export]
macro_rules! packet_set_devices{
    {$dev: expr, $value: expr, $nonce: expr} =>{
        [json!({
            "cmd": "SET_VOICE_SETTINGS",
            "args": {$dev:$value},
            "nonce": $nonce
        })]
    }
}

// Cairo helper
#[macro_export]
macro_rules! draw_overlay_gtk{
    {$window: expr, $ctx: expr, $avatar_list:expr, $state: expr} => {
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

                        let avatar_list = $avatar_list.lock().unwrap();
                        match user.avatar{
                            Some(_avatar) => {
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
                                                println!("Avatar ready but None {}",user.id );
                                            // Requested but no image (yet?) Don't draw anything more
                                            }
                                        }
                                    }
                                    None=>{
                                    }
                                }
                            },
                            None=>{}
                        }
                        if voice_state.deaf || voice_state.self_deaf {
                            draw_deaf($ctx, 0.0, y, line_height);
                        } else if voice_state.mute || voice_state.self_mute {
                            draw_mute($ctx, 0.0, y, line_height);
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

// Cairo helper
#[macro_export]
macro_rules! draw_overlay{
    {$ctx: expr, $avatar_list:expr, $state: expr} => {
        // Config / Static
        let edge = 6.0;
        let line_height = 32.0;

        $ctx.set_antialias(Antialias::Good);
        $ctx.set_operator(Operator::Source);
        $ctx.set_source_rgba(1.0, 0.0, 0.0, 0.0);
        $ctx.paint().expect("Unable to paint window");

        $ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
        $ctx.set_font_size(16.0);
        let state = $state.clone();
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

                        if voice_state.talking {
                            $ctx.set_source_rgba(0.0, 1.0, 0.0, 1.0);
                        } else {
                            $ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                        }
                        $ctx.show_text(&name).expect("unable to draw text");

                        let avatar_list = $avatar_list.lock().unwrap();
                        match user.avatar{
                            Some(_avatar) => {
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
                                                println!("Requested image but no data");
                                            // Requested but no image (yet?) Don't draw anything more
                                            }
                                        }
                                    }
                                    None=>{
                                    }
                                }
                            },
                            None=>{
                                println!("Error in userdata {:?}", user);
                            }
                        }
                        if voice_state.deaf || voice_state.self_deaf {
                            draw_deaf($ctx, 0.0, y, line_height);
                        } else if voice_state.mute || voice_state.self_mute {
                            draw_mute($ctx, 0.0, y, line_height);
                        }
                    }
                    None => {}
                }
                y += line_height;
            }
        }
    }
}
