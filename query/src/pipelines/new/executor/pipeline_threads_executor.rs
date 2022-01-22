// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;
use std::thread::JoinHandle;

use common_base::Thread;
use common_exception::ErrorCode;
use common_exception::Result;

use crate::pipelines::new::executor::pipeline_executor::PipelineExecutor;
use crate::pipelines::new::pipeline::NewPipeline;

#[allow(dead_code)]
pub struct PipelineThreadsExecutor {
    base: Option<Arc<PipelineExecutor>>,
    pipeline: Option<NewPipeline>,
    is_finished: bool,
    join_handles: Vec<JoinHandle<Result<()>>>,
}

#[allow(dead_code)]
impl PipelineThreadsExecutor {
    pub fn create(pipeline: NewPipeline) -> Result<PipelineThreadsExecutor> {
        Ok(PipelineThreadsExecutor {
            base: None,
            pipeline: Some(pipeline),
            is_finished: false,
            join_handles: vec![],
        })
    }

    pub fn execute(&mut self, workers: usize) -> Result<()> {
        if self.base.is_some() {
            return Err(ErrorCode::AlreadyStarted(
                "PipelineThreadsExecutor is already started.",
            ));
        }

        match self.pipeline.take() {
            None => Err(ErrorCode::LogicalError("Logical error: it's a bug.")),
            Some(pipeline) => self.start_workers(workers, pipeline),
        }
    }

    pub fn finish(&mut self) -> Result<()> {
        if !self.is_finished {
            self.is_finished = true;

            if let Some(executor) = &self.base {
                executor.finish();
            }

            while let Some(join_handle) = self.join_handles.pop() {
                // flatten error.
                match join_handle.join() {
                    Ok(Ok(_)) => Ok(()),
                    Ok(Err(cause)) => Err(cause),
                    Err(cause) => Err(ErrorCode::LogicalError(format!("{:?}", cause))),
                }?;
            }
        }

        Ok(())
    }

    fn start_workers(&mut self, workers: usize, pipeline: NewPipeline) -> Result<()> {
        let executor = PipelineExecutor::create(pipeline, workers)?;

        self.base = Some(executor.clone());
        for worker_num in 0..workers {
            let worker = executor.clone();
            self.join_handles.push(Thread::spawn(move || unsafe { worker.execute(worker_num) }));
        }

        Ok(())
    }
}
