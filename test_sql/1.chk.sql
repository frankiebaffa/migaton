select count(name)
from sqlite_master
where type = 'table'
and name = 'Tests'
limit 1;
