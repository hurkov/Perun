use std::path::Path;
use tokio::sync::mpsc::Receiver;

/// A single request to play a registered sound file.
#[derive(Debug, Clone)]
pub struct PlaybackCommand {
    pub title: String,
    pub path: String,
}

/// Drains the playback queue. Each command is played concurrently on its own
/// blocking task; failures are logged so one bad sound never stops the queue.
pub async fn run(mut receiver: Receiver<PlaybackCommand>) {
    while let Some(command) = receiver.recv().await {
        let _ = tokio::task::spawn_blocking(move || play_once(&command)).await;
    }
}

fn play_once(command: &PlaybackCommand) {
    let path = Path::new(&command.path);
    match attempt_play(path) {
        Ok(()) => tracing::info!(title = %command.title, ?path, "played sound"),
        Err(message) => tracing::warn!(
            title = %command.title,
            ?path,
            %message,
            "failed to play sound"
        ),
    }
}

fn attempt_play(path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("file not found: {e}"))?;
    // The OutputStream owns the mixer (Arc); the handle holds only a Weak
    // reference. `stream` must stay alive for the whole playback, otherwise
    // the sink can no longer find the mixer (PlayError::NoDevice).
    let (_stream, handle) =
        rodio::OutputStream::try_default().map_err(|e| format!("no audio output device: {e}"))?;
    let sink =
        rodio::Sink::try_new(&handle).map_err(|e| format!("failed to open audio device: {e}"))?;
    let source = rodio::Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("not valid audio: {e}"))?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
