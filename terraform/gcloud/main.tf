terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "5.30.0"
    }
  }
}

variable "cluster_name" {
  default = "test-cluster"
}

variable "project" {}

variable "region" {
  default = "us-west1"
}

variable "zone" {
  default = "us-west1-a"
}

###############################################################
# Set up the Networking Components
###############################################################
# Set GOOGLE_CREDENTIALS
provider "google" {
  region  = var.region
  project = var.project
}

provider "google-beta" {
  region = var.region
  zone   = var.zone
  project = var.project
}

resource "google_compute_network" "vpc_network" {
  name                    = "gke-vpc-network"
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "vpc_subnet" {
  name          = "k8s-network"
  ip_cidr_range = "10.3.0.0/16"
  region        = var.region
  network       = google_compute_network.vpc_network.id
}

###############################################################
# K8s configuration
###############################################################
resource "google_container_cluster" "cluster" {
  name             = var.cluster_name
  location         = var.zone

  # We can't create a cluster with no node pool defined, but we want to only use
  # separately managed node pools. So we create the smallest possible default
  # node pool and immediately delete it.
  remove_default_node_pool = true
  initial_node_count       = 1

  deletion_protection = false

  vertical_pod_autoscaling {
    enabled = true
  }

  workload_identity_config {
    workload_pool = "${var.project}.svc.id.goog"
  }
}

resource "google_container_node_pool" "larger_nodes" {
  name       = "larger-compute"
  location   = var.zone
  cluster    = google_container_cluster.cluster.name

  # enable_autopilot = true
  node_count = 4

  autoscaling {
    min_node_count = 3
    max_node_count = 4
  }

  node_config {
    preemptible  = true
    machine_type = "c2d-highmem-32"

    taint {
        effect = "NO_SCHEDULE"
        key    = "qdrant-node"
        value  = "present"
    }
  }
}

resource "google_container_node_pool" "simple_nodes" {
  name       = "simple-compute"
  location   = var.zone
  cluster    = google_container_cluster.cluster.name

  # enable_autopilot = true
  node_count = 3

  autoscaling {
    min_node_count = 1
    max_node_count = 3
  }

  node_config {
    preemptible  = true
    machine_type = "c2d-standard-8"
  }
}

resource "google_container_node_pool" "gpu_nodes" {
  name       = "gpu-compute"
  location   = var.zone
  cluster    = google_container_cluster.cluster.name
  node_count = 1

  autoscaling {
    min_node_count = 1
    max_node_count = 2
  }

  node_config {
    preemptible  = true
    machine_type = "g2-standard-4"

    gcfs_config {
      enabled = true   
    }

    gvnic {
      enabled = true
    }

    guest_accelerator {
      type  = "nvidia-l4"
      count = 1
      gpu_driver_installation_config {
        gpu_driver_version = "LATEST"
      }
      gpu_sharing_config {
        gpu_sharing_strategy       = "TIME_SHARING"
        max_shared_clients_per_gpu = 10
      }
    }

    workload_metadata_config {
      mode = "GCE_METADATA"
    }

    labels = {
      cluster_name = var.cluster_name
      purpose      = "gpu-time-sharing"
      node_pool    = "gpu-time-sharing"
    }

    taint {
        effect = "NO_SCHEDULE"
        key    = "nvidia.com/gpu"
        value  = "present"
    }

    tags = ["gke-my-project-id-region", "gke-my-project-id-region-gpu-time-sharing"]
  }
}
