use capataz::{Context, EventListener};
use tokio::sync::{Mutex, mpsc::{channel, Receiver}};
use std::sync::Arc;
use tokio::time;
use std::{path::{Path}};
use notify::{Event, RecommendedWatcher, recommended_watcher, Watcher, RecursiveMode};
use capataz::prelude::*;

fn file_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (tx, rx) = channel(10); // Using tokio's channel
    
    // Create a watcher with a synchronous callback
    let watcher = recommended_watcher(move |res| {
        // Create a clone of the sender for this callback invocation
        let tx_clone = tx.clone();
        
        // Since we're in a synchronous context, we can't use `.await`
        // Use a non-blocking send instead
        let _ = tx_clone.try_send(res).map_err(|e| {
            eprintln!("Error sending file event: {}", e);
        });
    })?;
    
    Ok((watcher, rx))
}

fn failing_branch<S, P>(
    path: P,
    name: S,
    sup_opts: Vec<supervisor::Opt>,
    worker_opts_factory: impl Fn() -> Vec<worker::Opt> + Send + Sync + 'static,
) -> supervisor::Spec
where
    S: Into<String>,
    P: AsRef<Path> + Clone + Send + Sync + 'static,
{
    supervisor::Spec::new_with_cleanup(name, sup_opts, move |_ctx| {
        // Create watcher and receiver in a more idiomatic way using ?.
        let (mut watcher, rx) = file_watcher()?;
        let rx = Arc::new(Mutex::new(rx));
        
        // Watch the path, using ? for error propagation.
        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

        // Create the worker specification.
        let worker_node = worker::Spec::new("worker", worker_opts_factory(), move |ctx| {
            let rx = rx.clone();
            async move {
                let mut rx = rx.lock().await;
                loop {
                    tokio::select! {
                        _ = ctx.done() => return Ok(()),
                        Some(event_result) = rx.recv() => {
                            event_result.map_err(|e| anyhow::anyhow!("Watcher error: {}", e))?;
                            return Err(anyhow::anyhow!("failure signal received"));
                        }
                        else => return Ok(())
                    }
                }
            }
        });

        // Create cleanup closure.
        let cleanup = {
            let path = path.clone();
            move || watcher.unwatch(path.as_ref()).map_err(anyhow::Error::new)
        };

        Ok((vec![worker_node], cleanup))
    })
}

#[tokio::main]
async fn main() {
    let (ctx, _cancel) = Context::new().with_cancel();

    let event_listener = EventListener::new(|ev| async move {
	println!("EVENT: {:?}", ev)
    });

    let spec = supervisor::Spec::new("root", vec![supervisor::with_strategy(supervisor::Strategy::OneForAll)], move |_ctx| {
	let b1 = failing_branch(
	    "./b1.txt",
	    "never_failing",
	    // branch restart tolerance.
	    vec![supervisor::with_restart_tolerance(5, time::Duration::from_secs(1))],
	    // worker options tolerance.
	    Vec::new);
	let b2 = failing_branch("./b2.txt", "b2", vec![], Vec::new);
	let b3 = failing_branch("./b3.txt", "b3", vec![], || vec![worker::with_restart(worker::Restart::Temporary)]);
	vec![b1.subtree(vec![]), b2.subtree(vec![]), b3.subtree(vec![])]
    });

    let result = spec.start(ctx, event_listener).await;
    match result {
	Ok(sup) =>  {
	    let (result, _spec) = sup.wait().await;
	    match result {
		Ok(_spec) => println!("application finished without errors"),
		Err(err) => println!("application finished with error: {}", err),
	    }
	},
	Err(err) => {
	    println!("application could not start due to error: {}", err);
	    return
	},
    }
}
