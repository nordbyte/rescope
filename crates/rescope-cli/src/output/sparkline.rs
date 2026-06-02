pub fn render(values: &[u64], width: usize) -> String {
    rescope_core::sparkline(values, width)
}
