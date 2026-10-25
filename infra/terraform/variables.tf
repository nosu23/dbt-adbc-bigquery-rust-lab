variable "project_id" {
  description = "Google Cloud project used for the BigQuery ADBC experiment."
  type        = string
}

variable "impersonator_member" {
  description = "IAM member allowed to impersonate the test service account, for example user:you@example.com."
  type        = string

  validation {
    condition = anytrue([
      startswith(var.impersonator_member, "user:"),
      startswith(var.impersonator_member, "serviceAccount:"),
    ])
    error_message = "impersonator_member must start with user: or serviceAccount:."
  }
}
