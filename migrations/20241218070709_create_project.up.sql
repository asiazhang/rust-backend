-- Add migration script here
create schema if not exists hm;

create table hm.projects
(
    id           serial primary key not null,
    -- 项目名称
    project_name varchar(255)       not null,
    -- 项目注释信息
    comment      varchar(255)       not null,
    created_at   timestamp          not null,
    updated_at    timestamp          not null
);

create extension if not exists pg_trgm;

-- 在project_name上使用gin类型索引，方便使用like %xxx% 这种模糊搜索
create index idx_project_search on hm.projects using gin (project_name gin_trgm_ops);