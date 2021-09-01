select count(name)
from pragma_table_info('TForeigns')
where name = 'Name'
limit 1;
