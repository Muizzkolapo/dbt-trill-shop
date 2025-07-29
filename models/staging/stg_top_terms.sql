with

source as (

    select * from {{ source('ecom', 'top_terms') }}

),

renamed as (

    select
        ----------  ids
        dma_id,
        
        ---------- text
        dma_name,
        term,
        
        ---------- dates
        refresh_date,
        week,
        
        ---------- numerics
        score,
        rank

    from source

)

select * from renamed