{% if !has_content %}No content extracted.{% else %}## Extracted Content

{% for item in data_items %}### {{ item.url }}

{% if item.has_markdown %}{{ item.markdown }}

{% endif %}---

{% endfor %}{% for item in error_items %}### {{ item.url }}

**Extraction failed:** {{ item.message }}

{% endfor %}{% endif %}
