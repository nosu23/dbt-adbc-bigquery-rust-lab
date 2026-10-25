output "service_account_email" {
  description = "Email address to pass as the BigQuery impersonation target."
  value       = google_service_account.no_storage_api_test.email
}

output "service_account_member" {
  description = "IAM member identifier of the test service account."
  value       = google_service_account.no_storage_api_test.member
}
