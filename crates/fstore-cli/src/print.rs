use chrono_humanize::{Accuracy, HumanTime, Tense};
use fstore::{Bucket, DateTime, Object, StoreTotals};
use log::debug;
use num_format::{SystemLocale, ToFormattedString};
use serde::Serialize;
use serde_json as json;
use std::io::{stdout, IsTerminal};
use tabled::{
    builder::Builder,
    settings::{object::Columns, Alignment, Padding, Reverse, Rotate, Style},
    Table,
};

fn locale() -> Option<SystemLocale> {
    match SystemLocale::default() {
        Ok(locale) => Some(locale),
        Err(err) => {
            debug!("Failed to load system locale: {err}");
            None
        }
    }
}

pub trait FormatDate {
    fn long_date(self) -> String;
}

impl FormatDate for DateTime {
    fn long_date(self) -> String {
        let formatted = self.format("%A, %B %-d, %Y at %r");
        let relative =
            HumanTime::from(self).to_text_en(Accuracy::Rough, Tense::Past);

        format!("{relative} on {formatted}")
    }
}

pub trait FormatNumber {
    fn format(&self) -> String;
}

impl FormatNumber for u64 {
    fn format(&self) -> String {
        match locale() {
            Some(locale) => self.to_formatted_string(&locale),
            None => format!("{self}"),
        }
    }
}

pub trait DiskUsage {
    fn disk_usage_string(self) -> String;
}

impl DiskUsage for u64 {
    fn disk_usage_string(self) -> String {
        if self < bytesize::KIB {
            format!("{} B", self)
        } else {
            format!(
                "{} ({} bytes)",
                bytesize::to_string(self, true),
                self.format()
            )
        }
    }
}

pub trait Tabulate {
    fn tabulate(self) -> Table;
}

impl Tabulate for Bucket {
    fn tabulate(self) -> Table {
        let mut builder = Builder::default();

        builder.push_record(["ID", "Name", "Created", "Objects", "Storage"]);

        builder.push_record([
            self.id.to_string(),
            self.name,
            self.created.long_date(),
            self.object_count.format(),
            self.space_used.disk_usage_string(),
        ]);

        let mut table = builder.build();

        table
            .with(Rotate::Left)
            .with(Reverse::rows())
            .modify(Columns::first(), Alignment::right())
            .with(Style::blank())
            .with(Padding::zero());

        table
    }
}

impl Tabulate for Vec<Bucket> {
    fn tabulate(self) -> Table {
        let mut builder = Builder::default();

        builder.push_record([
            "ID",
            "Name",
            "Date Created",
            "Objects",
            "Storage",
        ]);

        for bucket in self {
            builder.push_record([
                bucket.id.to_string(),
                bucket.name,
                bucket.created.to_string(),
                bucket.object_count.format(),
                bytesize::to_string(bucket.space_used, true),
            ]);
        }

        let mut table = builder.build();

        table
            .modify(Columns::new(3..), Alignment::right())
            .with(Style::modern_rounded());

        table
    }
}

impl Tabulate for Object {
    fn tabulate(self) -> Table {
        let media_type = self.media_type();

        let mut builder = Builder::default();

        builder.push_record(["ID", "SHA 256", "Size", "Type", "Added"]);

        builder.push_record([
            self.id.to_string(),
            self.hash,
            self.size.disk_usage_string(),
            media_type,
            self.added.long_date(),
        ]);

        let mut table = builder.build();

        table
            .with(Rotate::Left)
            .with(Reverse::rows())
            .modify(Columns::first(), Alignment::right())
            .with(Style::blank())
            .with(Padding::zero());

        table
    }
}

impl Tabulate for Vec<Object> {
    fn tabulate(self) -> Table {
        let mut builder = Builder::default();

        builder.push_record([
            "ID",
            "SHA 256",
            "Size",
            "Type",
            "Extension",
            "Added",
        ]);

        for object in self {
            let media_type = object.media_type();

            builder.push_record([
                object.id.to_string(),
                object.hash,
                bytesize::to_string(object.size, true),
                media_type,
                object.extension.unwrap_or_else(|| "?".into()),
                object.added.to_string(),
            ]);
        }

        let mut table = builder.build();

        table
            .modify(Columns::single(2), Alignment::right())
            .with(Style::modern_rounded());

        table
    }
}

impl Tabulate for StoreTotals {
    fn tabulate(self) -> Table {
        let mut builder = Builder::default();

        builder.push_record(["Buckets", "Objects", "Usage"]);

        builder.push_record([
            self.buckets.format(),
            self.objects.format(),
            self.space_used.disk_usage_string(),
        ]);

        let mut table = builder.build();

        table
            .with(Rotate::Left)
            .with(Reverse::rows())
            .modify(Columns::first(), Alignment::right())
            .with(Style::blank())
            .with(Padding::zero());

        table
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Output {
    pub human_readable: bool,
    pub json: bool,
}

pub trait Print {
    fn print(self, output: Output);
}

impl<T> Print for T
where
    T: Sized + Serialize + Tabulate,
{
    fn print(self, output: Output) {
        let print_json = || {
            println!("{}", json::to_string(&self).unwrap());
        };

        if output.json {
            print_json();

            if output.human_readable {
                eprintln!("{}", self.tabulate());
            }
        } else if output.human_readable || stdout().is_terminal() {
            println!("{}", self.tabulate());
        } else {
            print_json();
        }
    }
}
