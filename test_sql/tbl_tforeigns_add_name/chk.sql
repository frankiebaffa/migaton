select count(*)
from Test.sqlite_master
where name = 'TForeigns'
and type = 'table'
and sql like '%Name%';
