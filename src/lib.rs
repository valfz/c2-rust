pub mod admin;
pub mod implant;

pub mod proto {
    tonic::include_proto!("implant");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("implant_descriptor");
}