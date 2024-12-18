-- Add migration script here
create schema if not exists hm;

create table hm.projects
(
    id           serial primary key not null,
    project_name varchar(255)       not null
);

create extension if not exists pg_trgm;

create index idx_project_search on hm.projects using gin (project_name gin_trgm_ops);