begin transaction;
	if
	(
		select count(name)
		from sqlite_master
		where type = 'table'
		and name = 'TForeigns'
		limit 1
	) = 0
	begin;
		create table TForeigns
		(
			Id integer primary key autoincrement
		,	Test_Id integer not null
		,	foreign key (Test_Id) references Tests (Id)
		);
	end;
end transaction;

