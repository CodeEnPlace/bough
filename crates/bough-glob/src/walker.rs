use bough_fs::{Root, Twig};
use crate::Glob;
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::sync::Arc;

pub struct TwigWalker<'a, R: Root> {
    root: &'a R,
    includes: Vec<Glob>,
    excludes: Vec<Glob>,
}

enum WorkerMsg {
    Dir(PathBuf),
    File(Twig),
    Done,
}

impl<'a, R: Root> TwigWalker<'a, R> {
    pub fn new(root: &'a R) -> Self {
        Self {
            root,
            includes: Vec::new(),
            excludes: Vec::new(),
        }
    }

    pub fn include(mut self, glob: Glob) -> Self {
        self.includes.push(glob);
        self
    }

    pub fn exclude(mut self, glob: Glob) -> Self {
        self.excludes.push(glob);
        self
    }

    pub fn iter(self) -> TwigWalkerIter {
        let root_path = self.root.path().to_path_buf();
        let includes = Arc::new(self.includes);
        let excludes = Arc::new(self.excludes);

        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let (work_txs, work_rxs): (Vec<_>, Vec<_>) = (0..num_workers)
            .map(|_| crossbeam_channel::unbounded::<Option<PathBuf>>())
            .unzip();

        let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<WorkerMsg>();

        let mut worker_handles = Vec::with_capacity(num_workers);
        for work_rx in work_rxs {
            let root_path = root_path.clone();
            let includes = Arc::clone(&includes);
            let excludes = Arc::clone(&excludes);
            let msg_tx = msg_tx.clone();

            let handle = std::thread::spawn(move || {
                while let Ok(Some(dir)) = work_rx.recv() {
                    let Ok(entries) = std::fs::read_dir(&dir) else {
                        let _ = msg_tx.send(WorkerMsg::Done);
                        continue;
                    };

                    for entry in entries {
                        let Ok(entry) = entry else { continue };
                        let Ok(ft) = entry.file_type() else { continue };
                        let path = entry.path();

                        if ft.is_dir() || ft.is_symlink() {
                            let _ = msg_tx.send(WorkerMsg::Dir(path));
                        } else if ft.is_file() {
                            let Some(rel) = path.strip_prefix(&root_path).ok() else {
                                continue;
                            };
                            let inc = includes.iter().any(|g| g.is_match(rel));
                            let exc = excludes.iter().any(|g| g.is_match(rel));
                            if inc && !exc {
                                if let Ok(twig) = Twig::new(rel.to_path_buf()) {
                                    let _ = msg_tx.send(WorkerMsg::File(twig));
                                }
                            }
                        }
                    }
                    let _ = msg_tx.send(WorkerMsg::Done);
                }
            });
            worker_handles.push(handle);
        }
        drop(msg_tx);

        let (result_tx, result_rx) = crossbeam_channel::unbounded::<Twig>();

        let manager_handle = std::thread::spawn(move || {
            let mut pending: Vec<PathBuf> = vec![root_path];
            let mut in_flight: usize = 0;
            let mut next_worker: usize = 0;

            loop {
                while !pending.is_empty() && in_flight < num_workers {
                    let dir = pending.pop().unwrap();
                    let _ = work_txs[next_worker].send(Some(dir));
                    next_worker = (next_worker + 1) % num_workers;
                    in_flight += 1;
                }

                if in_flight == 0 {
                    for tx in &work_txs {
                        let _ = tx.send(None);
                    }
                    return;
                }

                let Ok(msg) = msg_rx.recv() else { return };
                match msg {
                    WorkerMsg::Dir(dir) => {
                        pending.push(dir);
                    }
                    WorkerMsg::File(twig) => {
                        let _ = result_tx.send(twig);
                    }
                    WorkerMsg::Done => {
                        in_flight -= 1;
                    }
                }
            }
        });

        TwigWalkerIter {
            result_rx,
            _manager: manager_handle,
            _workers: worker_handles,
        }
    }
}

pub struct TwigWalkerIter {
    result_rx: Receiver<Twig>,
    _manager: std::thread::JoinHandle<()>,
    _workers: Vec<std::thread::JoinHandle<()>>,
}

impl Iterator for TwigWalkerIter {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        self.result_rx.recv().ok()
    }
}
