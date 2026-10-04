use crate::{
    manager::{current_time_ms, manager},
    shape::Shape,
};
use pumpkin_plugin_api::{
    command::{
        Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, CommandSuggestion,
        CommandSuggestions, ConsumedArgs, StringType, SuggestionRequest,
    },
    commands::{CommandHandler, CommandSuggestionHandler},
    text::TextComponent,
    wit::pumpkin::plugin::command::Number,
    Server,
};

pub fn send_message(sender: &CommandSender, text: &str) {
    let component = TextComponent::from_legacy_string_with_code(text, '&');
    sender.send_message(component);
}

fn get_string_arg(args: &ConsumedArgs, key: &str) -> Option<String> {
    match args.get_value(key) {
        Arg::Simple(s) => Some(s),
        _ => None,
    }
}

fn get_int_arg(args: &ConsumedArgs, key: &str) -> Option<i64> {
    match args.get_value(key) {
        Arg::Num(Ok(Number::Int32(n))) => Some(n as i64),
        Arg::Num(Ok(Number::Int64(n))) => Some(n),
        _ => None,
    }
}

// --- Suggestion Handlers ---

pub struct ShapeSuggestionHandler;
impl CommandSuggestionHandler for ShapeSuggestionHandler {
    fn suggest(
        &self,
        _sender: CommandSender,
        _server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        let prefix = request.remaining.to_lowercase();
        let shapes = ["square", "circle", "diamond", "star"];
        let values = shapes
            .into_iter()
            .filter(|s| s.starts_with(&prefix))
            .map(|s| CommandSuggestion {
                value: s.to_string(),
                tooltip: None,
            })
            .collect();

        CommandSuggestions {
            start: request.start,
            length: request.remaining.len() as u32,
            values,
        }
    }
}

pub struct WorldSuggestionHandler;
impl CommandSuggestionHandler for WorldSuggestionHandler {
    fn suggest(
        &self,
        _sender: CommandSender,
        server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        let prefix = request.remaining.to_lowercase();
        let values = server
            .get_all_worlds()
            .into_iter()
            .map(|w| w.get_name())
            .filter(|name| name.to_lowercase().starts_with(&prefix))
            .map(|name| CommandSuggestion {
                value: name,
                tooltip: None,
            })
            .collect();

        CommandSuggestions {
            start: request.start,
            length: request.remaining.len() as u32,
            values,
        }
    }
}

// --- Command Handlers ---

pub struct ChunkyHelpHandler;
impl CommandHandler for ChunkyHelpHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let help = "\
&6--- Chunky Commands ---\n\
&e/chunky world [world] &7- Set or view target world\n\
&e/chunky shape <shape> &7- Set shape (square, circle, diamond, star)\n\
&e/chunky center [<x> <z>] &7- Set center coords (or current player pos)\n\
&e/chunky radius <radius> &7- Set generation radius in blocks\n\
&e/chunky worldborder &7- Set center and radius from world border\n\
&e/chunky selection &7- View current generation settings\n\
&e/chunky start &7- Start pre-generation task\n\
&e/chunky pause &7- Pause running pre-generation task\n\
&e/chunky continue &7- Resume paused pre-generation task\n\
&e/chunky cancel &7- Cancel pre-generation task\n\
&e/chunky progress &7- View current progress and CPS\n\
&e/chunky quiet <seconds> &7- Set progress update interval\n\
&e/chunky reload &7- Reload saved tasks and configuration";
        send_message(&sender, help);
        Ok(1)
    }
}

pub struct ChunkySelectionHandler;
impl CommandHandler for ChunkySelectionHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let summary = {
            let mgr = manager().lock().unwrap();
            mgr.selection.display_summary()
        };
        send_message(&sender, &summary);
        Ok(1)
    }
}

pub struct ChunkyWorldGetHandler;
impl CommandHandler for ChunkyWorldGetHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let mut mgr = manager().lock().unwrap();
        if let Some(w) = sender.world() {
            let world_name = w.get_name();
            mgr.selection.world = world_name.clone();
            mgr.save();
            send_message(
                &sender,
                &format!("&6[Chunky] &rTarget world set to &b{}&r.", world_name),
            );
        } else {
            send_message(
                &sender,
                &format!(
                    "&6[Chunky] &rCurrent target world is &b{}&r.",
                    mgr.selection.world
                ),
            );
        }
        Ok(1)
    }
}

pub struct ChunkyWorldSetHandler;
impl CommandHandler for ChunkyWorldSetHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if let Some(name) = get_string_arg(&args, "world") {
            let exists = server.has_world(&name);
            let mut mgr = manager().lock().unwrap();
            mgr.selection.world = name.clone();
            mgr.save();

            if exists {
                send_message(
                    &sender,
                    &format!("&6[Chunky] &rTarget world set to &b{}&r.", name),
                );
            } else {
                send_message(
                    &sender,
                    &format!(
                        "&6[Chunky] &rTarget world set to &b{}&r. &e(Note: world is not currently loaded)&r",
                        name
                    ),
                );
            }
        }
        Ok(1)
    }
}

pub struct ChunkyShapeHandler(pub Option<Shape>);
impl CommandHandler for ChunkyShapeHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let shape = if let Some(s) = self.0 {
            Some(s)
        } else if let Some(name) = get_string_arg(&args, "shape") {
            Shape::from_name(&name)
        } else {
            None
        };

        match shape {
            Some(s) => {
                let mut mgr = manager().lock().unwrap();
                mgr.selection.shape = s;
                let chunks = mgr.selection.estimated_chunks();
                mgr.save();
                send_message(
                    &sender,
                    &format!(
                        "&6[Chunky] &rShape set to &e{}&r (&a{}&r chunks).",
                        s.name(),
                        chunks
                    ),
                );
            }
            None => {
                send_message(
                    &sender,
                    "&c[Chunky] Invalid shape. Choose from: square, circle, diamond, star.",
                );
            }
        }
        Ok(1)
    }
}

pub struct ChunkyCenterCurrentHandler;
impl CommandHandler for ChunkyCenterCurrentHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if let Some(pos) = sender.position() {
            let cx = pos.0.floor() as i32;
            let cz = pos.2.floor() as i32;
            let mut mgr = manager().lock().unwrap();
            mgr.selection.center_x = cx;
            mgr.selection.center_z = cz;
            mgr.save();
            send_message(
                &sender,
                &format!("&6[Chunky] &rCenter set to &eX: {}, Z: {}&r.", cx, cz),
            );
        } else {
            send_message(
                &sender,
                "&c[Chunky] Console must specify coordinates: /chunky center <x> <z>",
            );
        }
        Ok(1)
    }
}

pub struct ChunkyCenterCoordsHandler;
impl CommandHandler for ChunkyCenterCoordsHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let x = get_int_arg(&args, "x");
        let z = get_int_arg(&args, "z");

        if let (Some(cx), Some(cz)) = (x, z) {
            let mut mgr = manager().lock().unwrap();
            mgr.selection.center_x = cx as i32;
            mgr.selection.center_z = cz as i32;
            mgr.save();
            send_message(
                &sender,
                &format!("&6[Chunky] &rCenter set to &eX: {}, Z: {}&r.", cx, cz),
            );
        } else {
            send_message(&sender, "&c[Chunky] Usage: /chunky center <x> <z>");
        }
        Ok(1)
    }
}

pub struct ChunkyRadiusHandler;
impl CommandHandler for ChunkyRadiusHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if let Some(r) = get_int_arg(&args, "radius") {
            if r <= 0 {
                send_message(&sender, "&c[Chunky] Radius must be greater than 0.");
                return Ok(1);
            }
            let mut mgr = manager().lock().unwrap();
            mgr.selection.radius = r as i32;
            let chunk_radius = (r + 15) / 16;
            let chunks = mgr.selection.estimated_chunks();
            mgr.save();
            send_message(
                &sender,
                &format!(
                    "&6[Chunky] &rRadius set to &e{}&r blocks (&e{}&r chunks, ~&a{}&r total chunks).",
                    r, chunk_radius, chunks
                ),
            );
        } else {
            send_message(&sender, "&c[Chunky] Usage: /chunky radius <radius>");
        }
        Ok(1)
    }
}

pub struct ChunkyWorldBorderHandler;
impl CommandHandler for ChunkyWorldBorderHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let target_world = {
            let mgr = manager().lock().unwrap();
            mgr.selection.world.clone()
        };

        let world = server
            .get_world_by_name(&target_world)
            .or_else(|| sender.world());

        if let Some(w) = world {
            let border = w.get_world_border();
            let cx = border.get_center_x().round() as i32;
            let cz = border.get_center_z().round() as i32;
            let size = border.get_size();
            let radius = (size / 2.0).round() as i32;

            let mut mgr = manager().lock().unwrap();
            mgr.selection.world = w.get_name();
            mgr.selection.center_x = cx;
            mgr.selection.center_z = cz;
            mgr.selection.radius = radius.max(16);
            let chunks = mgr.selection.estimated_chunks();
            mgr.save();

            send_message(
                &sender,
                &format!(
                    "&6[Chunky] &rCenter and radius set from world border (&eCenter: {}, {}&r, &eRadius: {}&r blocks, ~&a{}&r chunks).",
                    cx, cz, radius, chunks
                ),
            );
        } else {
            send_message(
                &sender,
                &format!(
                    "&c[Chunky] Target world '{}' not found to read world border.",
                    target_world
                ),
            );
        }
        Ok(1)
    }
}

pub struct ChunkyStartHandler;
impl CommandHandler for ChunkyStartHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let now_ms = current_time_ms();
        let result = {
            let mut mgr = manager().lock().unwrap();
            mgr.start_task(now_ms)
        };

        match result {
            Ok(msg) => {
                send_message(&sender, &msg);
                server.broadcast(&msg);
            }
            Err(err) => {
                send_message(&sender, &err);
            }
        }
        Ok(1)
    }
}

pub struct ChunkyPauseHandler;
impl CommandHandler for ChunkyPauseHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let result = {
            let mut mgr = manager().lock().unwrap();
            mgr.pause_task(&server)
        };

        match result {
            Ok(msg) => {
                send_message(&sender, &msg);
                server.broadcast(&msg);
            }
            Err(err) => {
                send_message(&sender, &err);
            }
        }
        Ok(1)
    }
}

pub struct ChunkyContinueHandler;
impl CommandHandler for ChunkyContinueHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let now_ms = current_time_ms();
        let result = {
            let mut mgr = manager().lock().unwrap();
            mgr.continue_task(now_ms)
        };

        match result {
            Ok(msg) => {
                send_message(&sender, &msg);
                server.broadcast(&msg);
            }
            Err(err) => {
                send_message(&sender, &err);
            }
        }
        Ok(1)
    }
}

pub struct ChunkyCancelHandler;
impl CommandHandler for ChunkyCancelHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let result = {
            let mut mgr = manager().lock().unwrap();
            mgr.cancel_task(&server)
        };

        match result {
            Ok(msg) => {
                send_message(&sender, &msg);
                server.broadcast(&msg);
            }
            Err(err) => {
                send_message(&sender, &err);
            }
        }
        Ok(1)
    }
}

pub struct ChunkyProgressHandler;
impl CommandHandler for ChunkyProgressHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let msg = {
            let mgr = manager().lock().unwrap();
            mgr.get_progress_message()
        };
        send_message(&sender, &msg);
        Ok(1)
    }
}

pub struct ChunkyQuietHandler;
impl CommandHandler for ChunkyQuietHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if let Some(sec) = get_int_arg(&args, "seconds") {
            let mut mgr = manager().lock().unwrap();
            mgr.config.quiet_interval_seconds = sec.max(0) as u64;
            mgr.save();
            send_message(
                &sender,
                &format!(
                    "&6[Chunky] &rQuiet interval set to &e{}&r seconds.",
                    mgr.config.quiet_interval_seconds
                ),
            );
        } else {
            send_message(&sender, "&c[Chunky] Usage: /chunky quiet <seconds>");
        }
        Ok(1)
    }
}

pub struct ChunkyReloadHandler;
impl CommandHandler for ChunkyReloadHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        {
            let mut mgr = manager().lock().unwrap();
            mgr.reload();
        }
        send_message(&sender, "&6[Chunky] &rConfiguration and state reloaded.");
        Ok(1)
    }
}

/// Builds and returns the complete Brigadier `/chunky` command tree.
pub fn build_chunky_command() -> Command {
    let world_arg_node = CommandNode::argument("world", &ArgumentType::String(StringType::SingleWord))
        .suggest(WorldSuggestionHandler)
        .execute(ChunkyWorldSetHandler);

    let world_node = CommandNode::literal("world")
        .execute(ChunkyWorldGetHandler)
        .then(world_arg_node);

    let shape_arg_node = CommandNode::argument("shape", &ArgumentType::String(StringType::SingleWord))
        .suggest(ShapeSuggestionHandler)
        .execute(ChunkyShapeHandler(None));

    let shape_node = CommandNode::literal("shape")
        .then(shape_arg_node)
        .then(CommandNode::literal("square").execute(ChunkyShapeHandler(Some(Shape::Square))))
        .then(CommandNode::literal("circle").execute(ChunkyShapeHandler(Some(Shape::Circle))))
        .then(CommandNode::literal("diamond").execute(ChunkyShapeHandler(Some(Shape::Diamond))))
        .then(CommandNode::literal("star").execute(ChunkyShapeHandler(Some(Shape::Star))));

    let center_z_node = CommandNode::argument("z", &ArgumentType::Integer((None, None)))
        .execute(ChunkyCenterCoordsHandler);
    let center_x_node = CommandNode::argument("x", &ArgumentType::Integer((None, None)))
        .then(center_z_node);
    let center_node = CommandNode::literal("center")
        .execute(ChunkyCenterCurrentHandler)
        .then(center_x_node);

    let radius_arg_node = CommandNode::argument("radius", &ArgumentType::Integer((Some(1), None)))
        .execute(ChunkyRadiusHandler);
    let radius_node = CommandNode::literal("radius").then(radius_arg_node);

    let quiet_arg_node = CommandNode::argument("seconds", &ArgumentType::Integer((Some(0), None)))
        .execute(ChunkyQuietHandler);
    let quiet_node = CommandNode::literal("quiet").then(quiet_arg_node);

    Command::new(
        &["chunky".to_string()],
        "Chunky chunk pre-generator for Pumpkin",
    )
    .execute(ChunkyHelpHandler)
    .then(CommandNode::literal("help").execute(ChunkyHelpHandler))
    .then(CommandNode::literal("selection").execute(ChunkySelectionHandler))
    .then(world_node)
    .then(shape_node)
    .then(center_node)
    .then(radius_node)
    .then(CommandNode::literal("worldborder").execute(ChunkyWorldBorderHandler))
    .then(CommandNode::literal("start").execute(ChunkyStartHandler))
    .then(CommandNode::literal("pause").execute(ChunkyPauseHandler))
    .then(CommandNode::literal("continue").execute(ChunkyContinueHandler))
    .then(CommandNode::literal("resume").execute(ChunkyContinueHandler))
    .then(CommandNode::literal("cancel").execute(ChunkyCancelHandler))
    .then(CommandNode::literal("progress").execute(ChunkyProgressHandler))
    .then(CommandNode::literal("status").execute(ChunkyProgressHandler))
    .then(quiet_node)
    .then(CommandNode::literal("reload").execute(ChunkyReloadHandler))
}
