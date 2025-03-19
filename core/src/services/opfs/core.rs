// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use crate::{Error, ErrorKind, Result};
use js_sys::Uint8Array;
use std::fmt::Debug;
use web_sys::{
    window, File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream,
};

use wasm_bindgen::{JsCast, JsValue};

use wasm_bindgen_futures::JsFuture;

fn parse_js_error(msg: JsValue) -> Error {
    Error::new(
        ErrorKind::Unexpected,
        msg.as_string().unwrap_or_else(String::new),
    )
}

struct OPFSFile {
    file: File,
    handle: FileSystemFileHandle,
}

async fn get_handle_by_filename(filename: &str) -> Result<FileSystemFileHandle, Error> {
    let navigator = window().unwrap().navigator();
    let storage_manager = navigator.storage();
    let root: FileSystemDirectoryHandle = JsFuture::from(storage_manager.get_directory())
        .await
        .and_then(JsCast::dyn_into)
        .map_err(parse_js_error)?;

    // maybe the option should be exposed?
    let opt = FileSystemGetFileOptions::new();
    opt.set_create(true);

    JsFuture::from(root.get_file_handle_with_options(filename, &opt))
        .await
        .and_then(JsCast::dyn_into)
        .map_err(parse_js_error)
}

impl OPFSFile {
    async fn from_filename(filename: &str) -> Result<Self> {
        let handle = get_handle_by_filename(filename).await?;
        Self::from_handle(handle).await
    }
    async fn from_handle(handle: FileSystemFileHandle) -> Result<Self> {
        let file: File = JsFuture::from(handle.get_file())
            .await
            .and_then(JsCast::dyn_into)
            .map_err(parse_js_error)?;

        Ok(Self { file, handle })
    }
    async fn read(&self, offset: u64, size: u64) -> Result<Uint8Array, Error> {
        let blob = self
            .file
            .slice_with_f64_and_f64(offset as f64, (size + offset) as f64)
            .map_err(parse_js_error)?;

        let array_buffer = JsFuture::from(blob.array_buffer())
            .await
            .map_err(parse_js_error)?;
        Ok(Uint8Array::new(&array_buffer))
    }
    async fn write(&self, content: &[u8], offset: u64) -> Result<(), Error> {
        let writable: FileSystemWritableFileStream = JsFuture::from(self.handle.create_writable())
            .await
            .and_then(JsCast::dyn_into)
            .map_err(parse_js_error)?;

        JsFuture::from(
            writable
                .seek_with_f64(offset as f64)
                .map_err(parse_js_error)?,
        )
        .await
        .map_err(parse_js_error)?;

        // QuotaExceeded or NotAllowed
        JsFuture::from(
            writable
                .write_with_u8_array(content)
                .map_err(parse_js_error)?,
        )
        .await
        .map_err(parse_js_error)?;

        JsFuture::from(writable.close())
            .await
            .map_err(parse_js_error)?;

        Ok(())
    }
    fn get_size(&self) -> u64 {
        self.file.size() as u64
    }
}

#[derive(Default, Debug)]
pub struct OpfsCore {}
