# Copyright 2025 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

variable "project" {
  type = string
}

variable "service_account_id" {
  type = string
}

provider "google" {
  alias   = "external_account_project"
  project = var.project
}

resource "google_service_account" "service_account" {
  provider     = google.external_account_project
  project      = var.project
  account_id   = var.service_account_id
  display_name = "External Account Test Service Account"
}

data "google_service_account" "build_runner_service_account" {
  account_id = "integration-test-runner"
}


resource "google_project_iam_member" "token_creator" {
  provider = google.external_account_project
  project  = var.project
  role     = "roles/iam.serviceAccountTokenCreator"
  member   = "serviceAccount:${data.google_service_account.build_runner_service_account.email}"
}
