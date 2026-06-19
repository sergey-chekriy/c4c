workspace "Removal" {
  model {
    user = person "User"
    system = softwareSystem "System"
    user -> system "Uses"
    user -/> system "Uses"
  }
}
