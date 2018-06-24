create table vendor (
    sequence_id SERIAL primary key,
    id UUID not null,
    org_sequence_id int not null,
    name varchar(255) null,
    google_place_id varchar(255) null,

    constraint fk_exp_organisation foreign key (org_sequence_id) references organisation (sequence_id)
);

CREATE UNIQUE INDEX idx_organisation_name ON vendor (name ASC NULLS LAST) WITH (FILLFACTOR = 50);