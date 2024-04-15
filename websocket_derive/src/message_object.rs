pub struct MessageObject<T>
{
    is_success: bool,
    command: String,
    object: Option<T>
}