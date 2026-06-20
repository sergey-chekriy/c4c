workspace "Invalid Documentation Paths" {
  !docs missing-docs
  !docs ../outside
  !docs /tmp/absolute-docs
  !docs "https://example.test/docs"
  !docs m7-content/unsupported.txt
  !adrs missing-adrs
  !adrs ../outside-adrs
  !adrs /tmp/absolute-adrs
  !adrs "https://example.test/adrs"
}
