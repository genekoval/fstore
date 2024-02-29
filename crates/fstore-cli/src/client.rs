use crate::{
    conf::Server,
    print::{DiskUsage, Output, Print, Tabulate},
};

use fstore::{http, ObjectError, Uuid};
use std::{error::Error, path::PathBuf, result};
use tokio::{
    fs::File,
    io::{stdin, stdout},
};
use tokio_util::io::StreamReader;

pub type BoxError = Box<dyn Error + Send + Sync + 'static>;
pub type Result = result::Result<(), BoxError>;

#[derive(Clone, Debug)]
pub struct Client {
    client: http::Client,
    output: Output,
}

impl Client {
    pub fn new(server: &Server, output: Output) -> Self {
        Self {
            client: http::Client::new(&server.url),
            output,
        }
    }

    pub async fn about(&self) -> Result {
        let about = self.client.about().await?;

        let version = &about.version;

        println!(
            r#"fstore server {url}
    Version {version}
        Branch {branch}
        Build Time {build_time}
        Build OS {build_os}
        Build Type {build_type}
        Commit Hash {commit_hash}
        Commit Date {commit_date}
        Rust Version {rust_version}
        Rust Channel {rust_channel}"#,
            url = self.client.url(),
            version = version.number,
            branch = version.branch,
            build_time = version.build_time,
            build_os = version.build_os,
            build_type = version.build_type,
            commit_hash = version.commit_hash,
            commit_date = version.commit_date,
            rust_version = version.rust_version,
            rust_channel = version.rust_channel,
        );

        Ok(())
    }

    pub async fn add_bucket(&self, name: String) -> Result {
        let bucket = self.client.add_bucket(&name).await?;

        println!("{}", bucket.id);

        Ok(())
    }

    pub async fn get_bucket(&self, name: String) -> Result {
        self.client.get_bucket(&name).await?.1.print(self.output);

        Ok(())
    }

    pub async fn get_buckets(&self) -> Result {
        self.client.get_buckets().await?.print(self.output);

        Ok(())
    }

    pub async fn get_object(
        &self,
        bucket: Uuid,
        object: Uuid,
        destination: Option<PathBuf>,
    ) -> Result {
        let stream = self.client.stream_object(&bucket, &object).await?;
        let mut reader = StreamReader::new(stream);

        match destination {
            Some(path) => {
                let mut file = File::options()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&path)
                    .await
                    .map_err(|err| {
                        format!(
                            "Failed to open file for writing '{}': {err}",
                            path.display()
                        )
                    })?;

                tokio::io::copy(&mut reader, &mut file).await.map_err(
                    |err| {
                        format!(
                            "Failed to stream object data to file '{}': {err}",
                            path.display()
                        )
                    },
                )?;
            }
            None => {
                tokio::io::copy(&mut reader, &mut stdout()).await.map_err(
                    |err| {
                        format!("Failed to stream object data to STDOUT: {err}")
                    },
                )?;
            }
        }

        Ok(())
    }

    pub async fn get_object_errors(&self) -> Result {
        let errors = self.client.get_object_errors().await?;

        for ObjectError { object_id, message } in &errors {
            println!("{object_id}");
            println!("\t{message}");
        }

        println!(
            "{} object error{}",
            errors.len(),
            match errors.len() {
                1 => "",
                _ => "s",
            }
        );

        Ok(())
    }

    pub async fn get_object_metadata(
        &self,
        bucket: Uuid,
        object: Uuid,
    ) -> Result {
        self.client
            .get_object(&bucket, &object)
            .await?
            .print(self.output);

        Ok(())
    }

    pub async fn prune(&self, print_objects: bool) -> Result {
        let objects = self.client.prune().await?;

        let total = objects.len();
        let reclaimed: u64 = objects.iter().map(|object| object.size).sum();

        match total {
            0 => println!("No objects to prune"),
            _ => {
                if print_objects {
                    println!("{}", objects.tabulate());
                }

                println!(
                    "Pruned {total} object{} freeing {}",
                    match total {
                        1 => "",
                        _ => "s",
                    },
                    reclaimed.disk_usage_string()
                );
            }
        }

        Ok(())
    }

    pub async fn remove_bucket(&self, id: Uuid) -> Result {
        self.client.remove_bucket(&id).await?;
        Ok(())
    }

    pub async fn remove_objects(
        &self,
        bucket: Uuid,
        objects: Vec<Uuid>,
    ) -> Result {
        let result = self.client.remove_objects(&bucket, &objects).await?;
        let total = result.objects_removed;

        match total {
            0 => println!("No objects were removed"),
            _ => {
                println!(
                    "Removed {total} object{} freeing {}",
                    match total {
                        1 => "",
                        _ => "s",
                    },
                    result.space_freed.disk_usage_string()
                );
            }
        }

        Ok(())
    }

    pub async fn rename_bucket(&self, id: &Uuid, name: &str) -> Result {
        Ok(self.client.rename_bucket(id, name).await?)
    }

    pub async fn status(&self) -> Result {
        self.client.status().await?.print(self.output);

        Ok(())
    }

    pub async fn stream_stdin(&self, bucket: String) -> Result {
        self.client
            .add_object(&bucket, stdin())
            .await?
            .print(self.output);

        Ok(())
    }

    pub async fn upload_file(&self, bucket: String, file: PathBuf) -> Result {
        let file = File::open(&file).await.map_err(|err| {
            format!("Failed to open file '{}': {err}", file.display())
        })?;

        self.client
            .add_object(&bucket, file)
            .await?
            .print(self.output);

        Ok(())
    }
}
