use hyperdeck_plugin::state::{AppState, GlobalHook};
use streamdeck_plugin::Plugin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hyperdeck_plugin::force_link();
    let state = AppState::default();
    Plugin::builder(std::env::args().skip(1))
        .state(state.clone())
        .lifecycle(GlobalHook { state })
        .add_registered_actions()
        .run()
        .await?;
    Ok(())
}
