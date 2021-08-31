begin transaction;
	if (
		select count(name)
		from sqlite_master
		where type = 'table'
		and name = 'Tests'
		limit 1
	) = 0
	begin
		create table Tests
		(
			Id integer primary key autoincrement
		,	Name text not null
		);
	end
end transaction;

