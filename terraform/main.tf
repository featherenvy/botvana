terraform {
  backend "pg" {}

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 3.27"
    }
  }

  required_version = ">= 0.14.9"
}

variable "cidr_vpc" {
  description = "CIDR block for the VPC"
  default     = "10.1.0.0/16"
}
variable "cidr_subnet" {
  description = "CIDR block for the subnet"
  default     = "10.1.0.0/24"
}
variable "availability_zone" {
  description = "availability zone to create subnet"
  default     = "eu-central-1a"
}


locals {
  name   = "botvana"
  region = "eu-central-1"
}

provider "aws" {
  profile = "default"
  region  = local.region
}

resource "aws_key_pair" "deployer" {
  key_name   = "deployer-key"
  public_key = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQDPuxzShkACrhis0Wjxc3IYw3iCQMh4VjTZGmzo2e9pTgnWVzujZa1hCtSw/5DvPoYv9riL0dTYq9Rl9s3k9AXQ9OybrtWJLCmzkIBL+Dp0S1HqQAdujmLX5/LbfkLtFz/RwewLs7mujvRFeCUvgAp1Ww4shriu5GUlgzHceWTPuuwR77BUekiwVpE5ExNtcVlzLGx/Kg2GEg7jFdkvGVTcSax+OtD1rbeYoNIcxL5AdtIONgq/87i+yjLPgPSY9zd7zfxCfJPnGYbSfErWZLze+/wp6JP3EORb22VX/f3nDnYSHCM/64F5mtqSiWYlj04qFNPBx3YysR2CJAzBq/u8fwFcBTu+jyavC1eei7ht52Er6LZYCqjzHgX2wlzyR+yjpgNArBs/+k8SNQ40rdM3kP3Zerqf+NSCSH/Csn5l1Oz/+q0rowAuPpCe37byilYAhqKCfVH8dD36KPBWO+W6rGLXWMkwKvKj5++5oNo6dY+vb1DDl40n5TjSg5sFoH0= aws_terraform_ssh_key"
}

resource "aws_vpc" "vpc" {
  cidr_block           = var.cidr_vpc
  enable_dns_support   = true
  enable_dns_hostnames = true
}

resource "aws_internet_gateway" "igw" {
  vpc_id = aws_vpc.vpc.id
}

resource "aws_subnet" "subnet_public" {
  vpc_id                  = aws_vpc.vpc.id
  cidr_block              = var.cidr_subnet
  map_public_ip_on_launch = "true"
  availability_zone       = var.availability_zone
}

resource "aws_route_table" "rtb_public" {
  vpc_id = aws_vpc.vpc.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.igw.id
  }
}

resource "aws_route_table_association" "rta_subnet_public" {
  subnet_id      = aws_subnet.subnet_public.id
  route_table_id = aws_route_table.rtb_public.id
}


resource "aws_security_group" "sg_22" {
  name   = "sg_22"
  vpc_id = aws_vpc.vpc.id
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_instance" "botvana" {
  ami           = "ami-0df357952f51264a2"
  instance_type = "t2.micro"
  key_name      = "deployer-key"

  subnet_id                   = aws_subnet.subnet_public.id
  vpc_security_group_ids      = ["${aws_security_group.sg_22.id}"]
  associate_public_ip_address = true

  tags = {
    Name = "bot"
  }
}
