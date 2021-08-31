begin transaction;
	if
	(
		select count(name)
		from pragma_table_info(Test)
		where name = 'Name'
		limit 1
	) = 0;
	begin;
		alter table TForeigns
		add column Name text not null;
	end;
end transaction;

