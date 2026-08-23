select * from (
    values
        ('ord_1', 'cust_1', 42.50, 'paid'),
        ('ord_2', 'cust_2', 17.00, 'pending'),
        ('ord_3', 'cust_1', 99.99, 'cancelled')
) as t(order_id, customer_id, amount, status)
