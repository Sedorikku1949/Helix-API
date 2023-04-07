#![deny(unsafe_code, unsafe_op_in_unsafe_fn, unstable_features, unstable_name_collisions)]
#![deny(clippy::unsafe_vector_initialization, clippy::unsafe_derive_deserialize, clippy::unsafe_removed_from_name)]
#![deny(clippy::unstable_as_mut_slice, clippy::unstable_as_slice, clippy::deprecated)]
#![deny(deprecated, deprecated_in_future)]
#![deny(deref_into_dyn_supertrait)]
#![deny(useless_deprecated)]

#![allow(dead_code)]
#![allow(unused_variables)]

use std::{ io::{ self, Write, Read }, fs::OpenOptions, str::FromStr, collections::HashMap, process::exit };
use chrono::{DateTime, Utc, NaiveDateTime};
use serde_json::{ self, Value, json };
use serde;

#[derive(Debug)]
pub enum SecurityAgentError {
  InvalidArchiveFilePath,
  InvalidArchive,
  CannotDecryptArchive,
  FailToReadArchiveFile,
  UnsafeArchiveFile,
  InvalidUtf8Translation,
  CannotWriteArchive
}

impl SecurityAgentError {
  #[allow(unreachable_patterns)]
  pub fn as_str(&self) -> &str {
    match self {
        Self::CannotDecryptArchive => "CannotDecryptArchive",
        Self::FailToReadArchiveFile => "FailToReadArchiveFile",
        Self::InvalidArchive => "InvalidArchive",
        Self::InvalidArchiveFilePath => "InvalidArchiveFilePath",
        Self::InvalidUtf8Translation => "InvalidUtf8Translation",
        Self::UnsafeArchiveFile => "UnsafeArchiveFile",
        Self::CannotWriteArchive => "CannotWriteArchive",
        _ => "Unknown"
    }
  }
}

const BLOAT: &str = "ThisMayBeABigTextOrNot";
const BLEP: i32 = 26;
const DATA_TYPE: &str = "json";
const MAGIC1: &[u8; 5] = &[127u8, 76u8, 69u8, 71u8, 82u8];
const MAGIC2: &[u8; 5] = &[127u8, 85u8, 69u8, 97u8, 127u8];
  
#[derive(Debug, Clone)]
pub struct ArchiveBody {
  pub data: Value
}

impl ArchiveBody {
  fn new(cnt: &String, _version: String, rewrite_on_error: bool) -> Result<Self, Self> {
    let json: Result<Value, _> = serde_json::from_str(cnt);
    match json {
      Ok(json) => Ok(Self { data: json }),
      Err(err) => {
        if !rewrite_on_error {
          exit(2);
        } else {
          Err(Self { data: Value::Object(serde_json::Map::new()) })
        }
      }
    }
  }
  fn format(&self) -> String {
    self.data.to_string()
  }
}

#[derive(Debug, Clone)]
pub struct ArchiveHeader {
  pub data_size: i32,
  pub creation: DateTime<Utc>,
  pub last_edited: DateTime<Utc>,
  pub version: String,
  pub bloat: String,
  pub data_type: String,
  pub owner_pid: i32
}

impl ArchiveHeader {
  fn new(raw_head: &String) -> Self {
    let mut interpreted: HashMap<String, Value> = HashMap::new();
    {
      let mut temp_n = String::new();
      let mut temp_v = String::new();
      let mut s = 0;
      for c in raw_head.split("") {
        if c == "{" || c == "}" {}
        else if c == "=" && s == 0 { s = 1 }
        else if c == "," && s == 1 {
          interpreted.insert(temp_n.clone(), Value::String(temp_v.clone()));
          s = 0;
          temp_n = String::new();
          temp_v = String::new();
        }
        else if s == 0 {
          temp_n.push_str(c as &str);
        } else {
          temp_v.push_str(c as &str);
        }
      }
    };
    let last_edited = {
      let binding = Value::String("0".to_string());
      let src = interpreted.get(&"last_edited".to_string()).unwrap_or(&binding).as_str().unwrap_or("0");
      let tmp: i64 = FromStr::from_str(src).unwrap_or(0);
      DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(tmp, 0).unwrap_or(NaiveDateTime::default()),
        Utc
      )
    };
    let creation = {
      let binding = Value::String("0".to_string());
      let src = interpreted.get(&"creation".to_string()).unwrap_or(&binding).as_str().unwrap_or("0");
      let tmp: i64 = FromStr::from_str(src).unwrap_or(0);
      DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(tmp, 0).unwrap_or(NaiveDateTime::default()),
        Utc
      )
    };
    let version = interpreted.get(&"version".to_string()).unwrap_or(&Value::String(BLOAT.clone().to_string())).to_string();
    let bloat = interpreted.get(&"bloat".to_string()).unwrap_or(&Value::String(BLOAT.clone().to_string())).to_string();
    let data_type = interpreted.get(&"data_type".to_string()).unwrap_or(&Value::String("raw".to_string())).to_string();
    Self {
        data_size: interpreted.get(&"data_size".to_string()).unwrap_or(&Value::String("0".to_string())).as_i64().unwrap_or(0_i64) as i32,
      creation,
      last_edited,
      version: version[1..version.len() - 1].to_string(),
      bloat: bloat[1..bloat.len() - 1].to_string(),
      data_type: data_type[1..data_type.len() - 1].to_string(),
      owner_pid: interpreted.get(&"owner_pid".to_string()).unwrap_or(&Value::String("0".to_string())).as_i64().unwrap_or(0_i64) as i32,
    }
  }
  fn format(&self) -> String {
    format!(
      "{{data_size={ds},creation={c},last_edited={le},version={v},bloat={s},data_type={dt},owner_pid={pid}}}",
      ds = self.data_size,
      c = self.creation.timestamp(),
      le = self.last_edited.timestamp(),
      v = self.version,
      s = self.bloat,
      dt = self.data_type,
      pid = self.owner_pid
    )
  }
}
#[derive(Debug, Clone)]
pub struct Archive {
  pub head: ArchiveHeader,
  pub body: ArchiveBody,
  pub path: String,
  auto_save: bool
}

impl Archive {
  pub fn new(save_path: &String, version: String, rewrite_on_err: bool, auto_save: bool) -> Self {
    let now = Utc::now();
    let head = ArchiveHeader::new(
      &format!(
        "{{data_size={ds},creation={c},last_edited={le},version={v},bloat={s},data_type={dt},owner_pid={pid}}}",
        ds = 0,
        c = now.timestamp(),
        le = now.timestamp(),
        v = version,
        s = BLOAT.clone().to_string(),
        dt = DATA_TYPE.clone().to_string(),
        pid = -1
      )
    );
    match ArchiveBody::new(&"{}".to_string(), version, rewrite_on_err) {
      Ok(body) => {
        Self { head, body, auto_save, path: save_path.clone().to_string() }
      },
      Err(body) => {
        Self { head, body, auto_save, path: save_path.clone().to_string() }
      }
    }
  }
  fn encrypt(data: &mut [u8], header_size: i32){
    for i in 0..data.len() as i32 {
      if i % 2 == 0 { data[i as usize] += ((header_size + i) % BLEP) as u8 }
      else { data[i as usize] -= ((header_size + i) % BLEP) as u8 };
    };
  }
  fn decrypt(data: &mut [u8], header_size: i32){
    for i in 0..data.len() as i32 {
      if i % 2 == 0 { data[i as usize] -= ((header_size + i) % BLEP) as u8 }
      else { data[i as usize] += ((header_size + i) % BLEP) as u8 };
    };
  }
  fn first_validity_check(bytes: &[u8]) -> bool {
    &bytes[0..MAGIC1.len()] == &MAGIC1[..]
  }
  fn second_validity_check(bytes: &[u8]) -> bool {
    &bytes[0..MAGIC1.len()] == &MAGIC2[..]
  }
  fn encrypt_data(&self) -> Vec<u8> {
    let header: String = self.head.format();
    let head_len = header.len() as i32;
    let body: String = self.body.format();
    let encrypted: &mut [u8] = &mut [&MAGIC2[..], format!("{}{}", header, body).as_bytes()].concat()[..];
    Archive::encrypt(
      encrypted,
      head_len
    );
    ([
      // DECRYPTED BY DEFAULT
      &MAGIC1[..],
      format!("{}", head_len).as_bytes(),
      b":",
      // ENCRYPTED
      encrypted
    ]).concat()
  }
  pub fn save(&self) -> Result<(), io::Error> {
    self.save_archive(true)
  }
  pub fn save_archive(&self, logs: bool) -> Result<(), io::Error> {
    let ftry = OpenOptions::new().create(true).truncate(true).read(true).write(true).open(&self.path);
    match ftry {
      Ok(mut f) => {
        let bytes = self.encrypt_data();
        if let Err(err) = f.write_all(&bytes[..]) {
            Err(err)
        }
        else {
            Ok(())
        }
      }
      Err(e) => Err(e)
    }
  }
  fn decrypt_data(raw: &mut [u8], path: &String, version: String, rewrite_on_err: bool, auto_save: bool) -> Result<Archive, SecurityAgentError> {
    if !Self::first_validity_check(raw) {
      return Err(SecurityAgentError::UnsafeArchiveFile);
    }
    let data = &raw.to_vec().to_owned().into_iter()
      .map(|c| std::str::from_utf8(&[c.to_owned()]).unwrap_or(&format!("{:?}", &c).as_str()).to_string())
      .collect::<Vec<String>>().join("");
    
    let header_size_pos: usize = (&data.find(":").unwrap_or(MAGIC1.len())).to_owned();
    let head_len: i32 = FromStr::from_str(&data[MAGIC1.len()..header_size_pos]).unwrap();
    let encrypted = &mut raw[(header_size_pos + 1)..];
    Self::decrypt(encrypted, head_len);
    if !Self::second_validity_check(encrypted) {
      return Err(SecurityAgentError::UnsafeArchiveFile)
    }
    // data certification ready
    let decrypted = &encrypted[MAGIC2.len()..];
    let head_raw = {
      match std::str::from_utf8(&decrypted[0..head_len as usize]) {
        Ok(r) => r,
        Err(e) => {
            return Err(SecurityAgentError::InvalidUtf8Translation)
        }
      }
    };
    let body_raw = {
      match std::str::from_utf8(&decrypted[head_len as usize..]) {
        Ok(r) => r,
        Err(e) => {
          return Err(SecurityAgentError::InvalidUtf8Translation)
        }
      }
    };
    let head = ArchiveHeader::new(&head_raw.to_string());
    let body = {
      let h = ArchiveBody::new(&body_raw.to_string(), version.clone(), rewrite_on_err);
      match h {
        Ok(ok) => ok,
        Err(krkr) => krkr
      }
    };
    Ok(Self { head, body, auto_save, path: path.clone().to_string() })
  }
  pub fn from_file(path: &String, version: String, rewrite_on_err: bool, auto_save: bool) -> Result<Self, SecurityAgentError> {
    let ftry = OpenOptions::new().create(false).read(true).write(false).open(path);
    match ftry {
      Ok(mut f) => {
        let mut buf: Vec<u8> = Vec::new();
        let bufread = f.read_to_end(&mut buf);
        if let Err(buferr) = bufread {
          Err(SecurityAgentError::FailToReadArchiveFile)
        } else {
         let s = Self::decrypt_data(&mut buf[..], path, version, rewrite_on_err, auto_save);
         s
        }
      },
      Err(err) => {
        Err(SecurityAgentError::InvalidArchiveFilePath)
      }
    }
  }
  pub fn try_load(path: &String, version: &String, rewrite_on_err: bool, auto_save: bool) -> Self {
    let tryed = Self::from_file(&path, version.clone(), rewrite_on_err.clone(), auto_save.clone());
    match tryed {
      Ok(a) => a,
      Err(_) => Self::new(&path, version.clone(), rewrite_on_err, auto_save)
    }
  }
  pub fn set<T>(&mut self, _origin: &str, k: &str, v: T) -> Result<(), SecurityAgentError>
  where
    T: serde::Serialize
  {
    self.body.data[k] = json!(v);
    // auto save
    if self.auto_save {
      match self.save_archive(false) {
        Ok(_) => Ok(()),
        Err(err) => {
          Err(SecurityAgentError::CannotWriteArchive)
        }
      }
    } else {
      Ok(())
    }
  }
  pub fn get(&self, _origin: &str, k: &str) -> Value {
    self.body.data[k].clone()
  }
}


pub mod medium_encryption {
    const BLOAT: u8 = 3;
    const MAX: u8 = 25;
  
    pub fn encrypt<T: AsRef<str>>(cnt: &T) -> String {
      let len = cnt.as_ref().len() as u8;
      cnt.as_ref().as_bytes()
        .iter()
        .map(|c| {
          (calc(c, &len, false) as char).to_string()
        })
        .collect::<Vec<String>>()
        .join("")
    }
    pub fn decrypt<T: AsRef<str>>(cnt: &T) -> String {
      let len = cnt.as_ref().len() as u8;
      cnt.as_ref().as_bytes()
        .iter()
        .map(|c| {
          (calc(c, &len, true) as char).to_string()
        })
        .collect::<Vec<String>>()
        .join("")
    }
  
    fn calc(c: &u8, len: &u8, reverse: bool) -> u8 {
      if c < &0u8 || c < &(len + BLOAT) { return *c };
      if reverse {
        // decrypt
        c + len - BLOAT
      } else {
        // encrypt
        c - len + BLOAT
      }
    }
  }