locals {
  required_services = toset([
    "bigquery.googleapis.com",
    "bigquerystorage.googleapis.com",
    "iam.googleapis.com",
    "iamcredentials.googleapis.com",
  ])
}

resource "google_project_service" "required" {
  for_each = local.required_services

  project            = var.project_id
  service            = each.value
  disable_on_destroy = false
}

resource "google_service_account" "no_storage_api_test" {
  project      = var.project_id
  account_id   = "dbt-adbc-no-storage-api-test"
  display_name = "dbt ADBC no Storage API test"
  description  = "Used to compare BigQuery ADBC drivers with and without the Storage Read API."

  depends_on = [google_project_service.required]
}

resource "google_project_iam_member" "bigquery_job_user" {
  project = var.project_id
  role    = "roles/bigquery.jobUser"
  member  = google_service_account.no_storage_api_test.member
}

# Intentionally do not grant roles/bigquery.readSessionUser. The missing
# bigquery.readsessions.* permissions are the condition under test.
resource "google_service_account_iam_member" "impersonator" {
  service_account_id = google_service_account.no_storage_api_test.name
  role               = "roles/iam.serviceAccountTokenCreator"
  member             = var.impersonator_member
}
